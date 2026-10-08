//! Part Forge: procedural parts and vehicles for warzone-5000.
//!
//! Pipeline: RON part file -> shape tree -> convex pieces -> voxel grid (mass, armour, internal volume) ->
//! render mesh with per-vertex attributes -> previews (PNG) and GLB. Vehicles are parts attached to sockets.
//! See docs/design/03-part-forge.md.

pub mod armour;
pub mod assemble;
pub mod build;
pub mod convex;
pub mod family;
pub mod geom;
pub mod gltf;
pub mod mesh;
pub mod preview;
pub mod raster;
pub mod schema;
pub mod voxel;

use std::collections::BTreeMap;
use std::path::Path;

use armour::ArmourTable;
use assemble::{power_balance_speed, Assembly, VehicleSheet};
use build::Piece;
use mesh::Mesh;
use schema::{DesignDef, Locomotion, MaterialLibrary, PartDef};
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
        let mut forge = Forge { lib, parts: BTreeMap::new(), designs: BTreeMap::new(), mass_cache: Default::default() };
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
    /// Each attachment's family sees its own sliders plus the hints of the socket it attaches to, so a track
    /// sizes itself to its hull. Identical requests share one generated part.
    pub fn instantiate(&mut self, d: &DesignDef) -> Result<DesignDef, String> {
        let families = family::all();
        let find = |id: &str| families.iter().find(|f| f.id() == id);
        let mut out = d.clone();
        if let Some(f) = find(&d.hull) {
            let def = f.generate(&family::values_for(f.as_ref(), &d.hull_params, &BTreeMap::new()));
            out.hull = self.add_generated(def)?;
        } else if !self.parts.contains_key(&d.hull) {
            return Err(format!("unknown hull '{}'", d.hull));
        }
        out.attach = self.instantiate_list(&out.hull.clone(), &d.attach, &families)?;
        Ok(out)
    }

    fn instantiate_list(&mut self, parent: &str, list: &[schema::Attach], families: &[Box<dyn family::Family>]) -> Result<Vec<schema::Attach>, String> {
        let mut out = Vec::new();
        for a in list {
            let mut a2 = a.clone();
            if let Some(fid) = &a.family {
                let f = families.iter().find(|f| f.id() == fid).ok_or_else(|| format!("unknown family '{fid}'"))?;
                let hints = self.parts[parent].def.sockets.iter().find(|s| s.name == a.socket).map(|s| s.hints.clone()).unwrap_or_default();
                let def = f.generate(&family::values_for(f.as_ref(), &a.params, &hints));
                a2.part = self.add_generated(def)?;
                a2.family = None;
            }
            if self.parts.contains_key(&a2.part) {
                a2.children = self.instantiate_list(&a2.part.clone(), &a.children, families)?;
            }
            out.push(a2);
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

    pub fn from_parts(lib: MaterialLibrary, parts: Vec<PartDef>) -> Result<Forge, String> {
        let mut forge = Forge { lib, parts: BTreeMap::new(), designs: BTreeMap::new(), mass_cache: Default::default() };
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
        let sheet = self.vehicle_sheet(&asm, &built);
        (built, asm, sheet)
    }

    fn vehicle_sheet(&self, asm: &Assembly, built: &Built) -> VehicleSheet {
        let mut s = VehicleSheet { mass_kg: built.mass.mass_kg, problems: asm.errors.clone(), ..Default::default() };
        let mut rolling = Vec::new();
        let mut efficiency = Vec::new();
        let mut gear_limit = f64::INFINITY;
        let mut disc_area = 0.0;
        let mut contact = 0.0;
        for (id, _) in &asm.parts {
            let f = &self.parts[id].def.function;
            s.power_kw += f.power_kw;
            s.draw_kw += f.draw_kw;
            s.load_kg += f.load_kg;
            disc_area += std::f64::consts::PI * f.rotor_radius_m * f.rotor_radius_m;
            contact += f.contact_m2;
            if let Some(l) = f.locomotion {
                if !s.locomotion.contains(&l) {
                    s.locomotion.push(l);
                }
                // Rolling resistance belongs to ground contact; flyers and hovering craft have none.
                let grounded = !matches!(l, Locomotion::Rotor | Locomotion::Jet | Locomotion::AntiGrav | Locomotion::Hover);
                rolling.push(if grounded { f.rolling.unwrap_or(0.03) } else { 0.0 });
                efficiency.push(assemble::drive_efficiency(l));
                gear_limit = gear_limit.min(f.max_kmh.unwrap_or(f64::INFINITY));
            }
        }
        // Frontal area: the silhouette seen from straight ahead.
        let row0 = armour::ELEVATIONS.iter().position(|e| *e == 0.0).unwrap();
        s.frontal_m2 = built.armour.rows[row0][0].area_m2;
        s.power_to_weight_kw_t = if s.mass_kg > 0.0 { s.power_kw / (s.mass_kg / 1000.0) } else { 0.0 };
        if s.locomotion.is_empty() {
            s.problems.push("no locomotion".into());
        }
        if s.power_kw <= 0.0 {
            s.problems.push("no engine".into());
        }
        if s.load_kg > 0.0 && s.mass_kg > s.load_kg {
            s.problems.push(format!("overloaded: {:.0} kg on locomotion rated {:.0} kg", s.mass_kg, s.load_kg));
        }
        if s.draw_kw > s.power_kw {
            s.problems.push(format!("power deficit: draws {:.0} kW of {:.0} kW", s.draw_kw, s.power_kw));
        }
        if contact > 0.0 {
            s.ground_pressure_kpa = s.mass_kg * 9.81 / contact / 1000.0;
        }
        let mean = |v: &[f64]| if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 };
        let weight_n = s.mass_kg * 9.81;
        let mut spare_kw = (s.power_kw - s.draw_kw).max(0.0);
        if s.locomotion.contains(&Locomotion::Rotor) {
            s.hover_kw = assemble::hover_power_w(weight_n, disc_area) / 1000.0;
            if s.hover_kw > spare_kw {
                s.problems.push(format!("cannot hover: needs {:.1} kW, has {:.1} kW", s.hover_kw, spare_kw));
            }
            spare_kw = (spare_kw - s.hover_kw).max(0.0);
        }
        let net_kw = spare_kw * mean(&efficiency);
        let c1 = mean(&rolling) * weight_n;
        let c3 = 0.5 * 1.225 * 0.9 * s.frontal_m2;
        let by_power = if s.locomotion.is_empty() { 0.0 } else { power_balance_speed(net_kw * 1000.0, c1, c3) * 3.6 };
        if by_power > gear_limit {
            s.top_speed_kmh = gear_limit;
            s.speed_limited_by = "running gear".into();
        } else {
            s.top_speed_kmh = by_power;
            s.speed_limited_by = "power".into();
        }
        s
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
