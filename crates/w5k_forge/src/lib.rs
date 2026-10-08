//! Part Forge: procedural parts and vehicles for warzone-5000.
//!
//! Pipeline: RON part file -> shape tree -> convex pieces -> voxel grid (mass, armour, internal volume) ->
//! render mesh with per-vertex attributes -> previews (PNG) and GLB. Vehicles are parts attached to sockets.
//! See docs/design/03-part-forge.md.

pub mod armour;
pub mod assemble;
pub mod build;
pub mod convex;
pub mod explore;
pub mod export;
pub mod family;
pub mod geom;
pub mod gltf;
pub mod mesh;
pub mod mover;
pub mod preview;
pub mod raster;
pub mod schema;
pub mod sheet;
pub mod voxel;

use std::collections::BTreeMap;
use std::path::Path;

use armour::ArmourTable;
use assemble::{Assembly, VehicleSheet};
use build::Piece;
use mesh::Mesh;
use schema::{DesignDef, MaterialLibrary, PartDef};
use voxel::{Grid, MassProps};

/// Default (minimum) voxel resolution along the longest axis.
pub const DEFAULT_VOXELS: u32 = 64;
/// Upper bound on the resolution chosen to resolve thin shells.
pub const MAX_VOXELS: u32 = 256;
/// Resolution of a whole vehicle's grid (used for bounds, free internal volume and ambient occlusion only).
pub const VEHICLE_VOXELS: u32 = 128;

/// A part (or a whole vehicle) turned into geometry and physical data.
#[derive(Clone, Debug)]
pub struct Built {
    pub pieces: Vec<Piece>,
    pub grid: Grid,
    pub mesh: Mesh,
    pub mass: MassProps,
    pub armour: ArmourTable,
}

#[derive(Clone, Debug)]
pub struct BuiltPart {
    pub def: PartDef,
    pub pieces: Vec<Piece>,
    /// Made by a parametric family for a design (not a content file).
    pub generated: bool,
}

/// Baked statistics written next to each part (and later loaded by the simulation).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct StatsFile {
    pub id: String,
    pub name: String,
    pub triangles: usize,
    pub voxel_cell_m: f64,
    pub mass: MassProps,
    pub armour: ArmourTable,
    #[serde(default)]
    pub vehicle: Option<VehicleSheet>,
}

/// Loaded content plus built part geometry.
pub struct Forge {
    pub lib: MaterialLibrary,
    pub parts: BTreeMap<String, BuiltPart>,
    pub designs: BTreeMap<String, DesignDef>,
    mass_cache: std::sync::Mutex<BTreeMap<String, MassProps>>,
    quick_cache: std::sync::Mutex<BTreeMap<String, (f64, geom::V3)>>,
}

fn read_ron<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    ron::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn ron_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut v: Vec<_> = std::fs::read_dir(dir)
        .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().map(|x| x == "ron").unwrap_or(false)).collect())
        .unwrap_or_default();
    v.sort();
    v
}

impl Forge {
    /// Load `materials.ron`, `parts/*.ron` and `vehicles/*.ron` from a content directory.
    pub fn load(content: &Path) -> Result<Forge, String> {
        let lib: MaterialLibrary = read_ron(&content.join("materials.ron"))?;
        let mut forge = Forge { lib, parts: BTreeMap::new(), designs: BTreeMap::new(), mass_cache: Default::default(), quick_cache: Default::default() };
        for f in ron_files(&content.join("parts")) {
            let def: PartDef = read_ron(&f)?;
            forge.add_part(def)?;
        }
        for f in ron_files(&content.join("vehicles")) {
            let d: DesignDef = read_ron(&f)?;
            let d = forge.instantiate(&d).map_err(|e| format!("{}: {e}", f.display()))?;
            forge.designs.insert(d.id.clone(), d);
        }
        Ok(forge)
    }

    /// Generate the parts a design asks for by family and return the design rewritten to use plain part ids.
    ///
    /// * Each attachment's family sees its own sliders plus the hints of the socket it attaches to, so a track
    ///   sizes itself to its hull. Identical requests share one generated part.
    /// * A socket name with `*` (`station_*`) attaches to every matching socket; left-side sockets (negative x
    ///   normal) are mirrored automatically.
    /// * A family only attaches to the socket kinds it declares (`Family::fits`).
    /// * The hull is raised so running gear with a `ride_height_m` (legs, air cushions, anti-gravity pods)
    ///   reaches the ground: the design's `lift_m`.
    pub fn instantiate(&mut self, d: &DesignDef) -> Result<DesignDef, String> {
        let families = family::all();
        let find = |id: &str| families.iter().find(|f| f.id() == id);
        let mut out = d.clone();
        out.lift_m = 0.0;
        if let Some(f) = find(&d.hull) {
            let def = f.generate(&family::values_for(f.as_ref(), &d.hull_params, &BTreeMap::new()), &self.lib);
            out.hull = self.add_generated(def)?;
        } else if !self.parts.contains_key(&d.hull) {
            return Err(format!("unknown hull '{}'", d.hull));
        }
        out.attach = self.instantiate_list(&out.hull.clone(), &d.attach, &families)?;
        let hull_sockets = self.parts[&out.hull].def.sockets.clone();
        let mut lift = 0.0f64;
        for a in &out.attach {
            let ride = self.parts.get(&a.part).and_then(|c| c.def.function.ride_height_m);
            let sock_y = hull_sockets.iter().find(|s| s.name == a.socket).map(|s| s.at[1]);
            if let (Some(ride), Some(y)) = (ride, sock_y) {
                lift = lift.max(ride - y);
            }
        }
        out.lift_m = lift;
        Ok(out)
    }

    fn instantiate_list(&mut self, parent: &str, list: &[schema::Attach], families: &[Box<dyn family::Family>]) -> Result<Vec<schema::Attach>, String> {
        let mut out = Vec::new();
        for a in list {
            let parent_sockets = self.parts[parent].def.sockets.clone();
            let wildcard = a.socket.contains('*');
            let targets: Vec<schema::SocketDef> = if wildcard {
                let star = a.socket.find('*').unwrap();
                let (pre, post) = (&a.socket[..star], &a.socket[star + 1..]);
                let m: Vec<_> = parent_sockets
                    .iter()
                    .filter(|s| s.name.len() >= pre.len() + post.len() && s.name.starts_with(pre) && s.name.ends_with(post))
                    .cloned()
                    .collect();
                if m.is_empty() {
                    return Err(format!("no socket matches '{}' on part '{}'", a.socket, parent));
                }
                m
            } else {
                parent_sockets.iter().filter(|s| s.name == a.socket).cloned().collect()
            };
            if targets.is_empty() {
                // Left for the assembler to report with the part's name.
                out.push(a.clone());
                continue;
            }
            for sock in targets {
                let mut a2 = a.clone();
                a2.socket = sock.name.clone();
                if wildcard && sock.normal[0] < -0.01 {
                    a2.mirror = true;
                }
                if let Some(fid) = &a.family {
                    let f = families.iter().find(|f| f.id() == fid).ok_or_else(|| format!("unknown family '{fid}'"))?;
                    if !f.fits().is_empty() && !f.fits().contains(&sock.kind) {
                        return Err(format!("family '{fid}' does not fit socket '{}' (a {:?} socket)", sock.name, sock.kind));
                    }
                    let def = f.generate(&family::values_for(f.as_ref(), &a.params, &sock.hints), &self.lib);
                    a2.part = self.add_generated(def)?;
                    a2.family = None;
                }
                if self.parts.contains_key(&a2.part) {
                    a2.children = self.instantiate_list(&a2.part.clone(), &a.children, families)?;
                }
                out.push(a2);
            }
        }
        Ok(out)
    }

    /// Add a generated part under an id derived from its content, so identical requests share it.
    fn add_generated(&mut self, mut def: PartDef) -> Result<String, String> {
        let mut h = w5k_math::StateHasher::new();
        h.write_bytes(ron::to_string(&def).map_err(|e| e.to_string())?.as_bytes());
        let id = format!("{}#{:08x}", def.id, h.finish() as u32);
        if !self.parts.contains_key(&id) {
            def.id = id.clone();
            self.add_part(def)?;
            self.parts.get_mut(&id).unwrap().generated = true;
        }
        Ok(id)
    }

    /// An empty forge with a material library (families and designs fill it as they are instantiated).
    pub fn new(lib: MaterialLibrary) -> Forge {
        Forge { lib, parts: BTreeMap::new(), designs: BTreeMap::new(), mass_cache: Default::default(), quick_cache: Default::default() }
    }

    pub fn from_parts(lib: MaterialLibrary, parts: Vec<PartDef>) -> Result<Forge, String> {
        let mut forge = Forge { lib, parts: BTreeMap::new(), designs: BTreeMap::new(), mass_cache: Default::default(), quick_cache: Default::default() };
        for p in parts {
            forge.add_part(p)?;
        }
        Ok(forge)
    }

    pub fn add_part(&mut self, def: PartDef) -> Result<(), String> {
        for p in collect_materials(&def) {
            if !self.lib.materials.contains_key(&p) {
                return Err(format!("part '{}' uses unknown material '{p}'", def.id));
            }
        }
        let pieces = build::build_part(&def);
        if pieces.is_empty() {
            return Err(format!("part '{}' produced no geometry", def.id));
        }
        if self.parts.contains_key(&def.id) {
            return Err(format!("duplicate part id '{}'", def.id));
        }
        self.parts.insert(def.id.clone(), BuiltPart { def, pieces, generated: false });
        Ok(())
    }

    pub fn part(&self, id: &str) -> Option<&BuiltPart> {
        self.parts.get(id)
    }

    /// Voxelise (at `res` cells along the longest axis), mesh and measure a set of pieces.
    pub fn measure(&self, pieces: Vec<Piece>, res: u32) -> Built {
        let grid = voxel::voxelise(&pieces, res);
        let mesh = mesh::build_mesh(&pieces, &grid);
        let mass = voxel::mass_props(&grid, &pieces, &self.lib);
        let armour = armour::armour_table(&pieces, &self.lib);
        Built { pieces, grid, mesh, mass, armour }
    }

    /// Voxel resolution for a part: its `voxels` setting (default 64) as a minimum, raised so the thinnest
    /// shell spans two cells (up to `MAX_VOXELS`).
    fn part_resolution(&self, p: &BuiltPart) -> u32 {
        voxel::resolution_for(&p.pieces, p.def.voxels.unwrap_or(DEFAULT_VOXELS), MAX_VOXELS)
    }

    pub fn build_part(&self, id: &str) -> Option<Built> {
        let p = self.part(id)?;
        Some(self.measure(p.pieces.clone(), self.part_resolution(p)))
    }

    /// Mass properties of a part alone (cached: vehicles reuse them).
    pub fn part_mass(&self, id: &str) -> Option<MassProps> {
        if let Some(m) = self.mass_cache.lock().unwrap().get(id) {
            return Some(m.clone());
        }
        let p = self.part(id)?;
        let grid = voxel::voxelise(&p.pieces, self.part_resolution(p));
        let m = voxel::mass_props(&grid, &p.pieces, &self.lib);
        self.mass_cache.lock().unwrap().insert(id.to_string(), m.clone());
        Some(m)
    }

    /// Mass and centre of mass of a part from exact piece volumes (no voxels), cached. Overlaps between pieces
    /// are counted twice, so this runs a few per cent heavy: good for sampling, not for the final sheet.
    fn quick_part_mass(&self, id: &str) -> (f64, geom::V3) {
        if let Some(v) = self.quick_cache.lock().unwrap().get(id) {
            return *v;
        }
        let p = &self.parts[id];
        let (mut m, mut first) = (0.0, geom::V3::ZERO);
        for piece in &p.pieces {
            let rho = self.lib.materials.get(&piece.mat).map(|x| x.density).unwrap_or(7850.0);
            let (v, c) = piece.material_volume();
            m += rho * v;
            first += c * (rho * v);
        }
        let r = (m, if m > 0.0 { first / m } else { geom::V3::ZERO });
        self.quick_cache.lock().unwrap().insert(id.to_string(), r);
        r
    }

    /// Evaluate a design fast: the assembly and its sheet without voxels, meshes or the full armour table
    /// (about a hundred times cheaper than `build_design`, within a few per cent on mass and armour).
    pub fn quick_design(&self, design: &DesignDef) -> (Assembly, VehicleSheet) {
        let asm = self.assemble(design);
        let (mut m, mut first) = (0.0, geom::V3::ZERO);
        for (id, x) in &asm.parts {
            let (pm, pc) = self.quick_part_mass(id);
            m += pm;
            first += x.point(pc) * pm;
        }
        let (lo, hi) = voxel::bounds(&asm.pieces);
        let inp = sheet::SheetInput {
            mass_kg: m,
            com: if m > 0.0 { first / m } else { geom::V3::ZERO },
            armour: armour::quick(&asm.pieces, &self.lib),
            lo,
            hi,
        };
        let sheet = self.make_sheet(&asm, &inp);
        (asm, sheet)
    }

    /// Assemble and measure a vehicle design. Returns the build, the assembly and the vehicle sheet.
    ///
    /// The vehicle's mass, centre of mass and inertia are **composed from its parts' own properties**, so
    /// the vehicle always weighs exactly the sum of its parts. The vehicle's own grid supplies bounds, free
    /// internal volume (internal parts such as engines take up their hull's interior) and ambient occlusion.
    pub fn build_design(&self, design: &DesignDef) -> (Built, Assembly, VehicleSheet) {
        let asm = self.assemble(design);
        let mut built = self.measure(asm.pieces.clone(), VEHICLE_VOXELS);
        let masses: Vec<(MassProps, geom::Xform)> =
            asm.parts.iter().filter_map(|(id, x)| self.part_mass(id).map(|m| (m, *x))).collect();
        let refs: Vec<(&MassProps, geom::Xform)> = masses.iter().map(|(m, x)| (m, *x)).collect();
        let mut mass = voxel::combine(&refs);
        mass.internal_m3 = built.mass.internal_m3;
        mass.bounds_min = built.mass.bounds_min;
        mass.bounds_max = built.mass.bounds_max;
        built.mass = mass;
        let inp = sheet::SheetInput {
            mass_kg: built.mass.mass_kg,
            com: geom::V3::from_arr(built.mass.centre_of_mass),
            armour: armour::ArmourSummary::from_table(&built.armour),
            lo: geom::V3::from_arr(built.mass.bounds_min),
            hi: geom::V3::from_arr(built.mass.bounds_max),
        };
        let sheet = self.make_sheet(&asm, &inp);
        (built, asm, sheet)
    }

    pub fn stats_file(&self, id: &str, name: &str, built: &Built, vehicle: Option<VehicleSheet>) -> StatsFile {
        StatsFile {
            id: id.to_string(),
            name: name.to_string(),
            triangles: built.mesh.triangle_count(),
            voxel_cell_m: built.grid.cell,
            mass: built.mass.clone(),
            armour: built.armour.clone(),
            vehicle,
        }
    }
}

fn collect_materials(def: &PartDef) -> Vec<String> {
    fn walk(n: &schema::Node, out: &mut Vec<String>) {
        use schema::Node::*;
        match n {
            Box { mat, .. } | Wedge { mat, .. } | Cylinder { mat, .. } | Sphere { mat, .. } | Beam { mat, .. } | Hull { mat, .. } => {
                if !out.contains(mat) {
                    out.push(mat.clone());
                }
            }
            Group { children, .. } | Mirror { children, .. } | Array { children, .. } | Radial { children, .. } => {
                for c in children {
                    walk(c, out);
                }
            }
        }
    }
    let mut out = Vec::new();
    for n in &def.shapes {
        walk(n, &mut out);
    }
    out
}
