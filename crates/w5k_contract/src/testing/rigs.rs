//! Dummy rigs: a valid `PhysRig` + `RenderRig` pair for a box truck and a box tank, with the complete node chains the real vehicles
//! will have (hull > travel > steer > wheel for every station; hull > turret > gun pitch > recoil for the tank). The viewers, Godot,
//! the AI and the validation harness can develop against these on day one. Not physics, not art.

use w5k_math::{scalar, Mat3, Quat, Transform, Vec3};

use crate::combat::CombatDef;
use crate::def::*;
use crate::param::Param;
use crate::render::*;
use crate::rig::*;
use crate::rig::{
    Actuator, BrakeLocation, BrakeSite, FireCycle, JointDrive, ProxyRole, RecoilDef, ServoDef, StabiliserDef, SteerLaw,
    WeaponDef, WheelContact,
};

// ------------------------------------------------------------------------------------------------------------ mesh helpers

/// An axis-aligned box centred at `c` with half extents `h`, 24 vertices with flat normals.
pub fn box_mesh(name: &str, node: usize, slot: usize, c: Vec3, h: Vec3) -> MeshPart {
    let (cs, hs) = (c.as_array(), h.as_array());
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    for a in 0..3 {
        let (u, v) = ((a + 1) % 3, (a + 2) % 3);
        for s in [1.0f64, -1.0] {
            let base = positions.len() as u32;
            for (su, sv) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                let mut p = cs;
                p[a] += s * hs[a];
                p[u] += su * hs[u];
                p[v] += sv * hs[v];
                positions.push([p[0] as f32, p[1] as f32, p[2] as f32]);
                let mut n = [0.0f32; 3];
                n[a] = s as f32;
                normals.push(n);
            }
            // u x v = +axis, so the (u, v) winding is CCW seen from +axis; flip for the -axis face.
            let quad = if s > 0.0 { [0, 1, 2, 0, 2, 3] } else { [0, 2, 1, 0, 3, 2] };
            indices.extend(quad.iter().map(|k| base + k));
        }
    }
    let n = positions.len();
    MeshPart {
        name: name.into(),
        node,
        material_slot: slot,
        positions,
        normals,
        edge: vec![0.0; n],
        cavity: vec![0.0; n],
        indices,
    }
}

/// A cylinder along local axis `axis` (0 = X, 1 = Y, 2 = Z) centred at `c`, with radial side normals and flat caps.
#[allow(clippy::too_many_arguments)] // a mesh generator's natural parameter list
pub fn cylinder_mesh(
    name: &str,
    node: usize,
    slot: usize,
    c: Vec3,
    radius: f64,
    half_len: f64,
    axis: usize,
    segments: usize,
) -> MeshPart {
    let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
    let cs = c.as_array();
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let point = |t: f64, along: f64| -> ([f32; 3], [f32; 3]) {
        let mut p = cs;
        let mut n = [0.0f32; 3];
        p[axis] += along;
        p[u] += radius * scalar::cos(t);
        p[v] += radius * scalar::sin(t);
        n[u] = scalar::cos(t) as f32;
        n[v] = scalar::sin(t) as f32;
        ([p[0] as f32, p[1] as f32, p[2] as f32], n)
    };
    // Side: a_i at -half_len, b_i at +half_len.
    for i in 0..=segments {
        let t = scalar::TAU * i as f64 / segments as f64;
        let (pa, na) = point(t, -half_len);
        let (pb, nb) = point(t, half_len);
        positions.push(pa);
        normals.push(na);
        positions.push(pb);
        normals.push(nb);
    }
    for i in 0..segments as u32 {
        let (a0, b0, a1, b1) = (2 * i, 2 * i + 1, 2 * i + 2, 2 * i + 3);
        indices.extend([a0, a1, b0, a1, b1, b0]);
    }
    // Caps.
    for (sign, along) in [(1.0f32, half_len), (-1.0f32, -half_len)] {
        let centre = positions.len() as u32;
        let mut cp = cs;
        cp[axis] += along;
        let mut cn = [0.0f32; 3];
        cn[axis] = sign;
        positions.push([cp[0] as f32, cp[1] as f32, cp[2] as f32]);
        normals.push(cn);
        let ring = positions.len() as u32;
        for i in 0..=segments {
            let t = scalar::TAU * i as f64 / segments as f64;
            let (p, _) = point(t, along);
            positions.push(p);
            normals.push(cn);
        }
        for i in 0..segments as u32 {
            if sign > 0.0 {
                indices.extend([centre, ring + i, ring + i + 1]);
            } else {
                indices.extend([centre, ring + i + 1, ring + i]);
            }
        }
    }
    let n = positions.len();
    MeshPart {
        name: name.into(),
        node,
        material_slot: slot,
        positions,
        normals,
        edge: vec![0.0; n],
        cavity: vec![0.0; n],
        indices,
    }
}

fn box_inertia(mass: f64, size: Vec3) -> Mat3 {
    let k = mass / 12.0;
    Mat3::diagonal(
        k * (size.y * size.y + size.z * size.z),
        k * (size.x * size.x + size.z * size.z),
        k * (size.x * size.x + size.y * size.y),
    )
}

fn solid(name: &str, mass_kg: f64, com: Vec3, size: Vec3) -> BodyDef {
    BodyDef { name: name.into(), mass_kg, com_m: com, inertia_kg_m2: box_inertia(mass_kg, size) }
}

// ----------------------------------------------------------------------------------------------------------------- shared

fn tyre() -> TyreDef {
    TyreDef {
        vertical_stiffness_n_m: 250_000.0,
        vertical_damping_ns_m: 1_500.0,
        mu_scale: 1.05,
        slip_stiffness: 12.0,
        cornering_stiffness_per_rad: 8.0,
        relaxation_length_m: 0.3,
        rolling_coeff: 0.015,
        inflation_pa: 240_000.0,
        patch_length_m: 0.18,
    }
}

fn damper(bump: f64, rebound: f64) -> DamperDef {
    DamperDef { bump_ns_m: bump, rebound_ns_m: rebound, knee_speed_m_s: 0.0, post_knee_ratio: 1.0, friction_n: 0.0 }
}

fn engine(curve: Vec<(f64, f64)>, idle: f64, redline: f64, inertia: f64, kind: EngineKind) -> EngineDef {
    EngineDef {
        kind,
        torque_curve: curve,
        idle_rpm: idle,
        redline_rpm: redline,
        inertia_kg_m2: inertia,
        drag_const_nm: 12.0,
        drag_per_rpm_nm: 0.006,
        bsfc_best_g_kwh: 240.0,
        free_output: false,
        response_time_s: 0.0,
        idle_fuel_kg_s: 0.0,
    }
}

fn brake(station: usize, max_torque_nm: f64, parking: bool) -> BrakeDef {
    BrakeDef {
        station,
        max_torque_nm,
        thermal_mass_j_k: 4_000.0,
        cooling_w_k: 8.0,
        cooling_per_ms_w_k: 4.0,
        fade_start_k: 600.0,
        fade_end_k: 900.0,
        fade_floor: 0.55,
        parking,
        service: true,
        location: BrakeLocation::AtStation,
        site: BrakeSite::Wheel,
        steering: false,
        reverse_torque_factor: 1.0,
        apply_time_s: 0.0,
        release_time_s: 0.0,
        circuit: 0,
    }
}

/// Where each kind of joint coordinate sits in the frame's `joints` vector (see `PhysRig::joint_names`).
struct Layout {
    stations: usize,
    steered: Vec<usize>,
}

impl Layout {
    fn of(rig: &PhysRig) -> Layout {
        Layout {
            stations: rig.stations.len(),
            steered: rig.stations.iter().enumerate().filter(|(_, s)| s.steer.is_some()).map(|(i, _)| i).collect(),
        }
    }
    fn spin(&self, station: usize) -> usize {
        station
    }
    fn steer(&self, station: usize) -> Option<usize> {
        self.steered.iter().position(|&s| s == station).map(|k| self.stations + k)
    }
    fn travel(&self, station: usize) -> usize {
        self.stations + self.steered.len() + station
    }
    fn articulation(&self, joint: usize) -> usize {
        2 * self.stations + self.steered.len() + joint
    }
}

fn spin_axis() -> Vec3 {
    // Positive spin = rolling forward (-Z). Rotation about -X by a positive angle carries the top of the wheel toward -Z.
    -Vec3::X
}

/// Add the node chain and meshes for one station: travel > (steer) > wheel.
#[allow(clippy::too_many_arguments)]
fn station_nodes(
    rr: &mut RenderRig,
    lay: &Layout,
    rig: &PhysRig,
    i: usize,
    tyre_slot: usize,
    rim_slot: usize,
    wheel_segments: usize,
) {
    let s = &rig.stations[i];
    let travel = rr.nodes.len();
    rr.nodes.push(RenderNode {
        name: format!("{}.travel", s.name),
        parent: Some(0),
        role: NodeRole::SuspensionArm,
        rest: Transform::from_pos(s.rest_pos_m),
        joint: Some(JointBinding { kind: JointAxisKind::Prismatic, axis: s.bump_dir, index: lay.travel(i) }),
    });
    let mut parent = travel;
    if let Some(k) = lay.steer(i) {
        let steer = rr.nodes.len();
        rr.nodes.push(RenderNode {
            name: format!("{}.steer", s.name),
            parent: Some(travel),
            role: NodeRole::SteerKnuckle,
            rest: Transform::IDENTITY,
            joint: Some(JointBinding { kind: JointAxisKind::Revolute, axis: Vec3::Y, index: k }),
        });
        parent = steer;
    }
    let wheel = rr.nodes.len();
    let role = match s.wheel.kind {
        WheelKind::Tyre => NodeRole::Wheel,
        WheelKind::RoadWheel => NodeRole::RoadWheel,
        WheelKind::Sprocket => NodeRole::Sprocket,
        WheelKind::Idler => NodeRole::Idler,
        WheelKind::ReturnRoller => NodeRole::ReturnRoller,
    };
    rr.nodes.push(RenderNode {
        name: format!("{}.wheel", s.name),
        parent: Some(parent),
        role,
        rest: Transform::IDENTITY,
        joint: Some(JointBinding { kind: JointAxisKind::Revolute, axis: spin_axis(), index: lay.spin(i) }),
    });
    let r = s.wheel.radius_m;
    let hw = 0.5 * s.wheel.width_m;
    rr.meshes.push(cylinder_mesh(&format!("{}.tyre", s.name), wheel, tyre_slot, Vec3::ZERO, r, hw, 0, wheel_segments));
    rr.meshes.push(cylinder_mesh(
        &format!("{}.rim", s.name),
        wheel,
        rim_slot,
        Vec3::ZERO,
        0.62 * r,
        hw * 1.05,
        0,
        wheel_segments,
    ));
    // A marker lug so that wheel spin is visible even on a plain cylinder.
    rr.meshes.push(box_mesh(
        &format!("{}.lug", s.name),
        wheel,
        rim_slot,
        Vec3::new(hw * 1.05, 0.0, 0.8 * r),
        Vec3::new(0.02, 0.04, 0.05),
    ));
}

fn finish(rr: &mut RenderRig, rig: &PhysRig) {
    rr.joint_count = rig.joint_names().len();
}

// -------------------------------------------------------------------------------------------------------------------- truck

/// A four-wheel-drive box truck: 2.4 t, 3 m wheelbase, 1.8 m track, 0.4 m wheels, 4 substeps. Hull datum at the hull box centre; the
/// ground is 0.75 m below it at rest.
pub fn box_truck() -> (PhysRig, RenderRig) {
    let (half_x, half_y, half_z) = (1.05, 0.5, 2.3);
    let mut stations = Vec::new();
    let spec = [
        ("fl", Side::Left, 0u8, -0.9, -1.5, true, 0usize),
        ("fr", Side::Right, 0, 0.9, -1.5, true, 1),
        ("rl", Side::Left, 1, -0.9, 1.5, false, 2),
        ("rr", Side::Right, 1, 0.9, 1.5, false, 3),
    ];
    for (name, side, axle, x, z, steered, out) in spec {
        stations.push(StationDef {
            name: name.into(),
            side,
            axle,
            rest_pos_m: Vec3::new(x, -0.35, z),
            bump_dir: Vec3::Y,
            bump_travel_m: 0.12,
            droop_travel_m: 0.12,
            unsprung_mass_kg: 55.0,
            suspension: SuspensionDef {
                spring: SpringKind::Linear { rate_n_m: 32_000.0 },
                preload_n: 4_900.0,
                damper: damper(2_200.0, 3_200.0),
                bump_stop: BumpStopDef {
                    engage_m: 0.09,
                    rate_n_m: 120_000.0,
                    progression: 1.0,
                    damping_ns_m: 0.0,
                    hard_limit: false,
                    restitution: 0.0,
                },
            },
            steer: steered.then_some(SteerDef { max_angle_rad: 0.55, ackermann: 1.0 }),
            wheel: WheelDef {
                kind: WheelKind::Tyre,
                radius_m: 0.4,
                width_m: 0.26,
                inertia_kg_m2: 1.4,
                tyre: Some(tyre()),
                patches_x_m: vec![],
            },
            drive_output: Some(out),
            arm_pivot_m: None,
        });
    }
    let axle_diff = |a: usize, b: usize| DriveNode::Diff {
        kind: DiffKind::Open,
        ratio: 3.73,
        bias: 0.0,
        split: vec![],
        efficiency: 1.0,
        children: vec![DriveNode::Output(a), DriveNode::Output(b)],
    };
    let drivetrain = DrivetrainDef {
        engine: engine(
            vec![(800.0, 150.0), (1500.0, 240.0), (2500.0, 300.0), (3500.0, 280.0), (4500.0, 220.0)],
            800.0,
            4500.0,
            0.35,
            EngineKind::Diesel,
        ),
        coupling: CouplingDef::TorqueConverter {
            stall_ratio: 2.0,
            k_factor_rpm_per_sqrt_nm: 85.0,
            lockup_speed_ratio: Some(0.85),
        },
        gearbox: GearboxDef {
            forward_ratios: vec![3.06, 1.63, 1.0, 0.7],
            reverse_ratios: vec![2.29],
            efficiency: 0.93,
            inertia_kg_m2: 0.12,
            shift: ShiftDef { automatic: true, upshift_rpm: 4_000.0, downshift_rpm: 1_600.0, shift_time_s: 0.4 },
        },
        driveline: DriveNode::Diff {
            kind: DiffKind::Open,
            ratio: 1.0,
            bias: 0.0,
            split: vec![],
            efficiency: 1.0,
            children: vec![axle_diff(0, 1), axle_diff(2, 3)],
        },
        outputs: (0..4).map(|i| OutputDef { station: i, final_drive_ratio: 1.0, efficiency: 0.97 }).collect(),
        brakes: vec![
            brake(0, 1_800.0, false),
            brake(1, 1_800.0, false),
            brake(2, 1_500.0, true),
            brake(3, 1_500.0, true),
        ],
        modes: vec![],
        default_mode: 0,
    };
    let rig = PhysRig {
        id: "box_truck".into(),
        // The datum is 0.7282 m above the ground: the free 0.4 m wheel centred 0.35 m below the datum overlaps it by the static tyre deflection (4,900 N sprung share + 55 kg unsprung over 250 kN/m = 21.8 mm).
        ride_height_m: 0.7282,
        linkages: vec![],
        combat: CombatDef::default(),
        hull: solid("hull", 2_000.0, Vec3::new(0.0, -0.05, 0.1), Vec3::new(2.0 * half_x, 2.0 * half_y, 2.0 * half_z)),
        stations,
        anti_roll: vec![
            AntiRollDef { left_station: 0, right_station: 1, rate_n_m: 9_000.0 },
            AntiRollDef { left_station: 2, right_station: 3, rate_n_m: 6_000.0 },
        ],
        tracks: vec![],
        drivetrain,
        articulation: vec![],
        aero: AeroDef { drag_coeff: 0.45, frontal_area_m2: 3.2, centre_of_pressure_m: Vec3::new(0.0, 0.2, 0.0) },
        proxies: vec![
            CollisionProxy {
                name: "hull".into(),
                shape: ProxyShape::Box { half_m: Vec3::new(half_x, half_y, half_z) },
                pose: Transform::IDENTITY,
                attached_to: None,
                attached_station: None,
                role: ProxyRole::Hull,
            },
            CollisionProxy {
                name: "cabin".into(),
                shape: ProxyShape::Box { half_m: Vec3::new(0.95, 0.4, 1.0) },
                pose: Transform::from_pos(Vec3::new(0.0, 0.9, -0.6)),
                attached_to: None,
                attached_station: None,
                role: ProxyRole::Hull,
            },
        ],
        muzzles: vec![],
        integration: IntegrationDef { substeps: 4 },
    };

    let lay = Layout::of(&rig);
    let mut rr = RenderRig {
        id: rig.id.clone(),
        nodes: vec![RenderNode {
            name: "hull".into(),
            parent: None,
            role: NodeRole::Hull,
            rest: Transform::IDENTITY,
            joint: None,
        }],
        meshes: vec![],
        material_slots: vec![
            MaterialSlot { name: "paint".into(), kind: SlotKind::Paint },
            MaterialSlot { name: "tyre".into(), kind: SlotKind::Rubber },
            MaterialSlot { name: "rim".into(), kind: SlotKind::Metal },
        ],
        joint_count: 0,
    };
    rr.meshes.push(box_mesh("hull", 0, 0, Vec3::ZERO, Vec3::new(half_x, half_y, half_z)));
    rr.meshes.push(box_mesh("cabin", 0, 0, Vec3::new(0.0, 0.9, -0.6), Vec3::new(0.95, 0.4, 1.0)));
    rr.meshes.push(box_mesh("hood_marker", 0, 2, Vec3::new(0.0, 0.52, -2.0), Vec3::new(0.5, 0.03, 0.25)));
    for i in 0..rig.stations.len() {
        station_nodes(&mut rr, &lay, &rig, i, 1, 2, 20);
    }
    finish(&mut rr, &rig);
    (rig, rr)
}

// --------------------------------------------------------------------------------------------------------------------- tank

/// A 40 t box tank: 7 m hull, 14 stations (idler, five road wheels, sprocket per side) on torsion bars, steering unit, two tracks,
/// and the full articulation chain (turret yaw > gun pitch > recoil) with a muzzle.
pub fn box_tank() -> (PhysRig, RenderRig) {
    let mut stations = Vec::new();
    let zs = [
        (-3.2, WheelKind::Idler, 0.35, "idler"),
        (-2.4, WheelKind::RoadWheel, 0.38, "r1"),
        (-1.2, WheelKind::RoadWheel, 0.38, "r2"),
        (0.0, WheelKind::RoadWheel, 0.38, "r3"),
        (1.2, WheelKind::RoadWheel, 0.38, "r4"),
        (2.4, WheelKind::RoadWheel, 0.38, "r5"),
        (3.3, WheelKind::Sprocket, 0.4, "spr"),
    ];
    for (side, sx, prefix) in [(Side::Left, -1.35, "l"), (Side::Right, 1.35, "r")] {
        for (k, (z, kind, radius, tag)) in zs.iter().enumerate() {
            let suspension = match kind {
                WheelKind::RoadWheel => SuspensionDef {
                    spring: SpringKind::Torsion { rate_nm_rad: 28_000.0, arm_length_m: 0.45, rest_arm_angle_rad: 0.35 },
                    // Ten road wheels carry the 44 t sprung mass (hull, turret, cradle, barrel): 431.5 kN / 10.
                    preload_n: 43_150.0,
                    damper: damper(18_000.0, 24_000.0),
                    bump_stop: BumpStopDef {
                        engage_m: 0.17,
                        rate_n_m: 4_000_000.0,
                        progression: 1.0,
                        damping_ns_m: 20_000.0,
                        hard_limit: false,
                        restitution: 0.0,
                    },
                },
                _ => SuspensionDef {
                    spring: SpringKind::Rigid,
                    preload_n: 0.0,
                    damper: damper(0.0, 0.0),
                    bump_stop: BumpStopDef {
                        engage_m: 0.0,
                        rate_n_m: 0.0,
                        progression: 0.0,
                        damping_ns_m: 0.0,
                        hard_limit: false,
                        restitution: 0.0,
                    },
                },
            };
            stations.push(StationDef {
                name: format!("{prefix}_{tag}"),
                side,
                axle: k as u8,
                rest_pos_m: Vec3::new(sx, -0.45, *z),
                bump_dir: Vec3::Y,
                bump_travel_m: if *kind == WheelKind::RoadWheel { 0.25 } else { 0.0 },
                droop_travel_m: if *kind == WheelKind::RoadWheel { 0.1 } else { 0.0 },
                // A rigid (spin-only) station is part of the hull: its mass is in `hull.mass_kg`.
                unsprung_mass_kg: if *kind == WheelKind::RoadWheel { 320.0 } else { 0.0 },
                suspension,
                steer: None,
                wheel: WheelDef {
                    kind: *kind,
                    radius_m: *radius,
                    width_m: 0.3,
                    inertia_kg_m2: 6.0,
                    tyre: None,
                    patches_x_m: vec![],
                },
                drive_output: if *kind == WheelKind::Sprocket {
                    Some(if side == Side::Left { 0 } else { 1 })
                } else {
                    None
                },
                arm_pivot_m: None,
            });
        }
    }
    let per_side = zs.len();
    let tracks: Vec<TrackDef> = [(Side::Left, 0usize), (Side::Right, per_side)]
        .iter()
        .map(|&(side, first)| TrackDef {
            name: if side == Side::Left { "track_l".into() } else { "track_r".into() },
            side,
            // Loop order: the sprocket, the top run to the idler, then the ground run back through the road wheels.
            stations: std::iter::once(first + per_side - 1).chain(first..first + per_side - 1).collect(),
            sprocket: first + per_side - 1,
            idler: first,
            width_m: 0.55,
            pitch_m: 0.15,
            contact_length_m: 4.8,
            mass_per_m_kg: 85.0,
            samples: 12,
            shoe_mu_scale: 1.0,
            shoe_mu_scale_soft: None,
            tension_n: 20_000.0,
            belt_length_m: 11.4,
            thickness_m: 0.07,
            resist_c0: 0.05,
            resist_c1_s_m: 0.0,
            sprocket_teeth: 11,
            wheel_contact: WheelContact { vertical_stiffness_n_m: 4_000_000.0, vertical_damping_ns_m: 20_000.0 },
        })
        .collect();
    let drivetrain = DrivetrainDef {
        engine: engine(
            vec![(600.0, 1_500.0), (1_200.0, 3_500.0), (1_800.0, 4_200.0), (2_400.0, 3_800.0), (2_800.0, 3_000.0)],
            600.0,
            2_800.0,
            3.0,
            EngineKind::Diesel,
        ),
        coupling: CouplingDef::TorqueConverter {
            stall_ratio: 2.4,
            k_factor_rpm_per_sqrt_nm: 40.0,
            lockup_speed_ratio: None,
        },
        gearbox: GearboxDef {
            forward_ratios: vec![4.0, 2.4, 1.5, 1.0],
            reverse_ratios: vec![4.4, 2.2],
            efficiency: 0.92,
            inertia_kg_m2: 1.2,
            shift: ShiftDef { automatic: true, upshift_rpm: 2_500.0, downshift_rpm: 1_100.0, shift_time_s: 0.6 },
        },
        driveline: DriveNode::SteerUnit {
            kind: SteerUnitKind::ControlledDifferential,
            ratio: 1.0,
            law: SteerLaw::default(),
            children: vec![DriveNode::Output(0), DriveNode::Output(1)],
        },
        outputs: vec![
            OutputDef { station: per_side - 1, final_drive_ratio: 5.0, efficiency: 0.96 },
            OutputDef { station: 2 * per_side - 1, final_drive_ratio: 5.0, efficiency: 0.96 },
        ],
        brakes: vec![brake(per_side - 1, 14_000.0, true), brake(2 * per_side - 1, 14_000.0, true)],
        modes: vec![],
        default_mode: 0,
    };
    let turret_anchor = Vec3::new(0.0, 0.6, 0.2);
    let gun_anchor = Vec3::new(0.0, 0.35, -1.4);
    let articulation = vec![
        JointDef {
            name: "turret_yaw".into(),
            role: JointRole::TurretYaw,
            parent: None,
            kind: JointKind::Revolute,
            anchor_m: turret_anchor,
            axis: Vec3::Y,
            limits: None,
            body: solid("turret", 12_000.0, Vec3::new(0.0, 0.5, 0.1), Vec3::new(2.8, 0.9, 3.6)),
            drive: JointDrive::Servo(ServoDef {
                max_rate_si: 0.7,
                // 40 kN m over the 53,458 kg m^2 of turret, cradle and barrel gives at most 0.748 rad/s^2.
                max_accel_si: 0.7,
                max_effort_si: 40_000.0,
                kp_si: 60_000.0,
                kd_si: 80_000.0,
                actuator: Actuator::Hydraulic,
                latency_s: 0.05,
                fallback_rate_si: 0.0,
                stabiliser: Some(StabiliserDef { rejection: 0.9, bandwidth_hz: 3.0, latency_s: 0.01 }),
            }),
            aim_channel: 0,
        },
        JointDef {
            name: "gun_pitch".into(),
            role: JointRole::GunPitch,
            parent: Some(0),
            kind: JointKind::Revolute,
            anchor_m: gun_anchor,
            axis: Vec3::X,
            limits: Some((-0.14, 0.35)),
            body: solid("gun_cradle", 2_800.0, Vec3::new(0.0, 0.0, -0.6), Vec3::new(0.5, 0.5, 1.5)),
            drive: JointDrive::Servo(ServoDef {
                max_rate_si: 0.35,
                max_accel_si: 0.9,
                max_effort_si: 12_000.0,
                kp_si: 30_000.0,
                kd_si: 26_000.0,
                actuator: Actuator::Hydraulic,
                latency_s: 0.05,
                fallback_rate_si: 0.0,
                stabiliser: Some(StabiliserDef { rejection: 0.9, bandwidth_hz: 3.0, latency_s: 0.01 }),
            }),
            aim_channel: 0,
        },
        JointDef {
            name: "gun_recoil".into(),
            role: JointRole::Recoil,
            parent: Some(1),
            kind: JointKind::Prismatic,
            anchor_m: Vec3::ZERO,
            axis: Vec3::Z,
            limits: Some((0.0, 0.35)),
            body: solid("barrel", 1_200.0, Vec3::new(0.0, 0.0, -2.5), Vec3::new(0.15, 0.15, 5.0)),
            // A passive recuperator: a spring that holds the barrel in battery (above m g sin(35 deg) = 4 kN) and a buffer.
            drive: JointDrive::Recoil(RecoilDef {
                stroke_m: 0.35,
                spring: SpringKind::Linear { rate_n_m: 150_000.0 },
                preload_n: 20_000.0,
                damper_ns_m: 30_000.0,
                damper_quad_ns2_m2: 0.0,
            }),
            aim_channel: 0,
        },
    ];
    let rig = PhysRig {
        id: "box_tank".into(),
        // Road-wheel bottoms (centre -0.45, radius 0.38) plus the 0.07 m belt put the ground 0.90 m below the datum.
        ride_height_m: 0.90,
        linkages: vec![],
        combat: CombatDef::default(),
        hull: solid("hull", 28_000.0, Vec3::new(0.0, -0.15, 0.2), Vec3::new(3.0, 1.2, 7.0)),
        stations,
        anti_roll: vec![],
        tracks,
        drivetrain,
        articulation,
        aero: AeroDef { drag_coeff: 0.9, frontal_area_m2: 8.0, centre_of_pressure_m: Vec3::new(0.0, 0.6, 0.0) },
        proxies: vec![
            CollisionProxy {
                name: "hull".into(),
                shape: ProxyShape::Box { half_m: Vec3::new(1.5, 0.6, 3.5) },
                pose: Transform::IDENTITY,
                attached_to: None,
                attached_station: None,
                role: ProxyRole::Hull,
            },
            CollisionProxy {
                name: "turret".into(),
                shape: ProxyShape::Box { half_m: Vec3::new(1.4, 0.4, 1.8) },
                pose: Transform::from_pos(Vec3::new(0.0, 0.4, 0.1)),
                attached_to: Some(0),
                attached_station: None,
                role: ProxyRole::Other,
            },
        ],
        muzzles: vec![MuzzleDef {
            name: "main_gun".into(),
            joint: Some(2),
            pose: Transform::new(Vec3::new(0.0, 0.0, -4.9), Quat::IDENTITY),
            caliber_m: 0.12,
            // Placeholder numbers (an 8 kg shell at 1.65 km/s): the right order of magnitude, not a dossier.
            weapon: WeaponDef {
                catalogue_id: "stand_in_120mm".into(),
                trigger: 0,
                projectile_mass_kg: 8.0,
                muzzle_velocity_m_s: 1_650.0,
                recoil_impulse_factor: 1.5,
                dispersion_mrad: 0.3,
                cycle: FireCycle::Single { reload_s: 6.0 },
            },
        }],
        integration: IntegrationDef { substeps: 6 },
    };

    let lay = Layout::of(&rig);
    let mut rr = RenderRig {
        id: rig.id.clone(),
        nodes: vec![RenderNode {
            name: "hull".into(),
            parent: None,
            role: NodeRole::Hull,
            rest: Transform::IDENTITY,
            joint: None,
        }],
        meshes: vec![],
        material_slots: vec![
            MaterialSlot { name: "paint".into(), kind: SlotKind::Paint },
            MaterialSlot { name: "rubber".into(), kind: SlotKind::Rubber },
            MaterialSlot { name: "metal".into(), kind: SlotKind::Metal },
            MaterialSlot { name: "track".into(), kind: SlotKind::Track },
        ],
        joint_count: 0,
    };
    rr.meshes.push(box_mesh("hull", 0, 0, Vec3::ZERO, Vec3::new(1.5, 0.6, 3.5)));
    for i in 0..rig.stations.len() {
        station_nodes(&mut rr, &lay, &rig, i, 1, 2, 18);
    }
    for (side, sx, name) in [(Side::Left, -1.35, "track_l"), (Side::Right, 1.35, "track_r")] {
        let _ = side;
        let n = rr.nodes.len();
        rr.nodes.push(RenderNode {
            name: name.into(),
            parent: Some(0),
            role: NodeRole::Track,
            rest: Transform::from_pos(Vec3::new(sx, -0.45, 0.05)),
            joint: None,
        });
        rr.meshes.push(box_mesh(name, n, 3, Vec3::ZERO, Vec3::new(0.27, 0.43, 3.7)));
    }
    // Turret > gun pitch > recoil.
    let turret = rr.nodes.len();
    rr.nodes.push(RenderNode {
        name: "turret_yaw".into(),
        parent: Some(0),
        role: NodeRole::Turret,
        rest: Transform::from_pos(turret_anchor),
        joint: Some(JointBinding { kind: JointAxisKind::Revolute, axis: Vec3::Y, index: lay.articulation(0) }),
    });
    rr.meshes.push(box_mesh("turret", turret, 0, Vec3::new(0.0, 0.4, 0.1), Vec3::new(1.4, 0.4, 1.8)));
    let gun = rr.nodes.len();
    rr.nodes.push(RenderNode {
        name: "gun_pitch".into(),
        parent: Some(turret),
        role: NodeRole::GunPitch,
        rest: Transform::from_pos(gun_anchor),
        joint: Some(JointBinding { kind: JointAxisKind::Revolute, axis: Vec3::X, index: lay.articulation(1) }),
    });
    rr.meshes.push(box_mesh("mantlet", gun, 0, Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.45, 0.35, 0.3)));
    let barrel = rr.nodes.len();
    rr.nodes.push(RenderNode {
        name: "gun_recoil".into(),
        parent: Some(gun),
        role: NodeRole::Recoil,
        rest: Transform::IDENTITY,
        joint: Some(JointBinding { kind: JointAxisKind::Prismatic, axis: Vec3::Z, index: lay.articulation(2) }),
    });
    rr.meshes.push(cylinder_mesh("barrel", barrel, 2, Vec3::new(0.0, 0.0, -2.45), 0.07, 2.45, 2, 16));
    finish(&mut rr, &rig);
    (rig, rr)
}

// ------------------------------------------------------------------------------------------------------------- VehicleDef

/// A `VehicleDef` for the box truck, every number an `Estimate` marked as a stand-in. It checks clean and round-trips through RON,
/// which is all it is for: FORGE's real reference vehicles (HMMWV, M113, ...) replace it.
pub fn dummy_vehicle_def() -> VehicleDef {
    let e = |v: f64, lo: f64, hi: f64| Param::estimate(v, lo, hi, "stand-in value, not a measurement");
    let axle = |from_front: f64, steered: bool| AxleDef {
        from_front_m: e(from_front, from_front - 0.1, from_front + 0.1),
        track_width_m: e(1.8, 1.6, 2.0),
        steered,
        driven: true,
        anti_roll_n_m: Some(e(8_000.0, 2_000.0, 20_000.0)),
    };
    VehicleDef {
        id: "box_truck".into(),
        name: "Box truck (stand-in)".into(),
        reference: None,
        hull: HullDef {
            length_m: e(4.6, 4.4, 4.8),
            width_m: e(2.1, 2.0, 2.2),
            height_m: e(1.8, 1.7, 1.9),
            mass_kg: e(2_000.0, 1_800.0, 2_300.0),
            com_height_m: e(0.7, 0.55, 0.9),
            com_from_front_m: e(2.2, 2.0, 2.5),
            ground_clearance_m: e(0.25, 0.2, 0.3),
        },
        running_gear: RunningGearDef::Wheeled(WheeledDef {
            axles: vec![axle(0.8, true), axle(3.8, false)],
            tyre: TyreSliders {
                outer_diameter_m: e(0.8, 0.7, 0.9),
                section_width_m: e(0.26, 0.2, 0.32),
                inflation_pa: e(240_000.0, 150_000.0, 350_000.0),
                mu_peak_ref: e(0.95, 0.8, 1.1),
                cornering_stiffness_per_rad: e(8.0, 5.0, 12.0),
                rolling_coeff: e(0.015, 0.01, 0.03),
                unsprung_mass_kg: e(55.0, 40.0, 90.0),
            },
        }),
        suspension: SuspensionSliders {
            kind: SuspensionKind::Coil,
            front_ride_frequency_hz: e(1.6, 1.2, 2.2),
            rear_ride_frequency_hz: e(1.7, 1.2, 2.4),
            damping_ratio: e(0.3, 0.2, 0.5),
            bump_travel_m: e(0.12, 0.08, 0.2),
            droop_travel_m: e(0.12, 0.08, 0.2),
        },
        powertrain: PowertrainDef {
            engine: EngineSliders {
                kind: EngineKind::Diesel,
                peak_power_w: e(110_000.0, 90_000.0, 140_000.0),
                peak_power_rpm: e(3_500.0, 3_000.0, 4_000.0),
                peak_torque_nm: e(300.0, 250.0, 350.0),
                peak_torque_rpm: e(2_500.0, 2_000.0, 3_000.0),
                idle_rpm: e(800.0, 700.0, 900.0),
                redline_rpm: e(4_500.0, 4_000.0, 5_000.0),
                inertia_kg_m2: e(0.35, 0.2, 0.6),
                bsfc_best_g_kwh: e(240.0, 200.0, 280.0),
                torque_curve: None,
            },
            coupling: CouplingSliders::TorqueConverter { stall_ratio: e(2.0, 1.6, 2.6), lockup: true },
            gearbox: GearboxSliders {
                forward_ratios: [3.06, 1.63, 1.0, 0.7].iter().map(|&r| e(r, r * 0.97, r * 1.03)).collect(),
                reverse_ratios: vec![e(2.29, 2.2, 2.4)],
                efficiency: e(0.93, 0.9, 0.96),
                automatic: true,
                upshift_rpm: e(4_000.0, 3_500.0, 4_400.0),
                downshift_rpm: e(1_600.0, 1_200.0, 2_000.0),
                shift_time_s: e(0.4, 0.2, 0.8),
            },
            final_drive_ratio: e(3.73, 3.0, 4.6),
            transfer_case_ratio: Some(e(1.0, 1.0, 1.0)),
            steering_unit: None,
            driveline_efficiency: e(0.9, 0.85, 0.95),
        },
        brakes: BrakesDef {
            service_decel_g: e(0.8, 0.6, 0.9),
            front_share: e(0.6, 0.5, 0.7),
            thermal_mass_kj_k: e(4.0, 2.0, 8.0),
            parking_brake: true,
        },
        aero: AeroSliders { drag_coeff: e(0.45, 0.35, 0.6), frontal_area_m2: e(3.2, 2.8, 3.6) },
        turret: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn joint_bindings(rr: &RenderRig) -> Vec<usize> {
        let mut v: Vec<usize> = rr.nodes.iter().filter_map(|n| n.joint.map(|j| j.index)).collect();
        v.sort_unstable();
        v
    }

    #[test]
    fn both_rigs_validate_and_masses_are_plausible() {
        for (rig, rr) in [box_truck(), box_tank()] {
            rig.validate().unwrap_or_else(|e| panic!("{}: {e:?}", rig.id));
            rr.validate().unwrap_or_else(|e| panic!("{}: {e:?}", rr.id));
            assert!(rig.total_mass_kg() > 2_000.0);
            assert!(rr.triangle_count() > 100);
        }
        let (truck, _) = box_truck();
        let (tank, _) = box_tank();
        assert!((truck.total_mass_kg() - 2_220.0).abs() < 1.0);
        assert!(tank.total_mass_kg() > 40_000.0 && tank.total_mass_kg() < 50_000.0, "{}", tank.total_mass_kg());
    }

    #[test]
    fn render_joint_bindings_cover_the_joint_layout_exactly_once() {
        for (rig, rr) in [box_truck(), box_tank()] {
            let names = rig.joint_names();
            assert_eq!(rr.joint_count, names.len());
            let used = joint_bindings(&rr);
            // Every station spin and travel, every steer, every articulation joint is bound exactly once.
            assert_eq!(used, (0..names.len()).collect::<Vec<_>>(), "{}", rig.id);
        }
    }

    #[test]
    fn the_wheels_stand_on_the_ground_at_the_design_ride_height() {
        let (truck, _) = box_truck();
        let lowest = truck.stations.iter().map(|s| s.rest_pos_m.y - s.wheel.radius_m).fold(f64::MAX, f64::min);
        assert!((lowest + 0.75).abs() < 1e-12);
        // Static check: the preload carries the sprung weight: 4 x 4.9 kN ~ 2000 kg x g.
        let load: f64 = truck.stations.iter().map(|s| s.suspension.preload_n).sum();
        assert!((load - truck.hull.mass_kg * scalar::G).abs() / load < 0.01);
    }

    #[test]
    fn the_dummy_vehicle_def_checks_clean_and_round_trips_through_ron() {
        let d = dummy_vehicle_def();
        d.check().unwrap_or_else(|e| panic!("{e:?}"));
        let text = ron::ser::to_string_pretty(&d, ron::ser::PrettyConfig::default()).expect("serialise");
        let back: VehicleDef = ron::from_str(&text).expect("parse");
        assert_eq!(d, back);
        let (spec, measured, estimate, tuned) = d.provenance_counts();
        assert_eq!((spec, measured, tuned), (0, 0, 0));
        assert!(estimate > 40);
    }

    #[test]
    fn box_mesh_normals_point_outward() {
        let m = box_mesh("b", 0, 0, Vec3::new(1.0, 2.0, 3.0), Vec3::new(0.5, 0.6, 0.7));
        let c = Vec3::new(1.0, 2.0, 3.0);
        for t in m.indices.chunks(3) {
            let p: Vec<Vec3> = t
                .iter()
                .map(|&i| {
                    Vec3::new(
                        m.positions[i as usize][0] as f64,
                        m.positions[i as usize][1] as f64,
                        m.positions[i as usize][2] as f64,
                    )
                })
                .collect();
            let n = (p[1] - p[0]).cross(p[2] - p[0]);
            let centroid = (p[0] + p[1] + p[2]) * (1.0 / 3.0);
            assert!(n.dot(centroid - c) > 0.0, "triangle winds inward");
        }
    }

    #[test]
    fn cylinder_triangles_wind_outward() {
        let c = Vec3::new(0.0, 0.0, 0.0);
        for axis in 0..3 {
            let m = cylinder_mesh("c", 0, 0, c, 0.5, 1.0, axis, 12);
            for t in m.indices.chunks(3) {
                let p: Vec<Vec3> = t
                    .iter()
                    .map(|&i| {
                        Vec3::new(
                            m.positions[i as usize][0] as f64,
                            m.positions[i as usize][1] as f64,
                            m.positions[i as usize][2] as f64,
                        )
                    })
                    .collect();
                let n = (p[1] - p[0]).cross(p[2] - p[0]);
                if n.length() < 1e-12 {
                    continue;
                }
                let centroid = (p[0] + p[1] + p[2]) * (1.0 / 3.0);
                assert!(n.dot(centroid - c) > -1e-9, "axis {axis}: triangle winds inward");
            }
        }
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;
    use w5k_math::scalar;

    fn errors(rig: &PhysRig) -> Vec<String> {
        rig.validate().err().unwrap_or_default()
    }

    fn breaks(what: &str, mut rig: PhysRig, mutate: impl Fn(&mut PhysRig), needle: &str) {
        mutate(&mut rig);
        let e = errors(&rig);
        assert!(e.iter().any(|m| m.contains(needle)), "{what}: expected an error containing {needle:?}, got {e:?}");
    }

    #[test]
    fn validate_rejects_every_kind_of_broken_rig_and_says_why() {
        let (truck, _) = box_truck();
        let (tank, _) = box_tank();
        breaks("sprocket index", tank.clone(), |r| r.tracks[0].sprocket = 999, "out of range");
        breaks(
            "steer unit children reversed",
            tank.clone(),
            |r| {
                if let DriveNode::SteerUnit { children, .. } = &mut r.drivetrain.driveline {
                    children.reverse();
                }
            },
            "[left, right]",
        );
        breaks(
            "no static equilibrium",
            tank.clone(),
            |r| r.stations.iter_mut().for_each(|s| s.suspension.preload_n = 0.0),
            "static equilibrium",
        );
        breaks(
            "duplicate station names",
            truck.clone(),
            |r| r.stations[1].name = r.stations[0].name.clone(),
            "duplicate station name",
        );
        breaks(
            "negative inertia",
            truck.clone(),
            |r| r.hull.inertia_kg_m2 = Mat3::diagonal(-1.0, -1.0, 1.0),
            "positive-definite",
        );
        breaks(
            "recoil limits contradict the stroke",
            tank.clone(),
            |r| r.articulation[2].limits = Some((0.0, 0.2)),
            "limits must be (0, stroke_m)",
        );
        breaks(
            "servo acceleration the effort cannot deliver",
            tank.clone(),
            |r| {
                if let JointDrive::Servo(s) = &mut r.articulation[0].drive {
                    s.max_accel_si = 5.0;
                }
            },
            "exceeds what max_effort_si allows",
        );
        breaks(
            "bore along the recoil axis",
            tank.clone(),
            |r| {
                r.muzzles[0].pose =
                    Transform::new(Vec3::new(0.0, 0.0, -4.9), w5k_math::Quat::from_axis_angle(Vec3::Y, scalar::PI))
            },
            "opposite to the recoil axis",
        );
        breaks("too many substeps", truck.clone(), |r| r.integration.substeps = 1000, "substeps");
        breaks("no ride height", truck.clone(), |r| r.ride_height_m = 0.0, "ride_height_m");
        breaks("wheel buried in the ground", truck.clone(), |r| r.ride_height_m = 0.5, "overlaps");
        breaks(
            "rigid station carrying mass",
            tank.clone(),
            |r| r.stations[0].unsprung_mass_kg = 10.0,
            "part of the hull",
        );
        breaks(
            "a table that disagrees with its preload",
            truck.clone(),
            |r| r.stations[0].suspension.spring = SpringKind::Table { points: vec![(-0.1, 1_000.0), (0.1, 12_000.0)] },
            "TOTAL force",
        );
        breaks(
            "a linkage member that has its own spring",
            truck.clone(),
            |r| {
                let sus = r.stations[0].suspension.clone();
                r.linkages.push(LinkageDef {
                    name: "beam".into(),
                    members: vec![(0, 0.5), (1, 0.5)],
                    suspension: sus,
                    rock_limit_m: None,
                });
            },
            "must have SpringKind::Rigid",
        );
        breaks(
            "a negative anti-roll rate",
            truck.clone(),
            |r| r.anti_roll[0].rate_n_m = -1.0,
            "rate must be finite and >= 0",
        );
        breaks(
            "a track that does not start at the sprocket",
            tank.clone(),
            |r| r.tracks[0].stations.rotate_left(1),
            "loop order",
        );
        assert!(truck.validate().is_ok() && tank.validate().is_ok());
    }

    #[test]
    fn optional_features_are_listed_so_a_solver_can_refuse_what_it_does_not_implement() {
        let (truck, _) = box_truck();
        let (tank, _) = box_tank();
        assert!(truck.required_features().is_empty());
        assert!(tank.required_features().contains(&crate::rig::feature::TRACK_RESISTANCE));
        let mut t = truck.clone();
        t.stations[0].wheel.patches_x_m = vec![-0.15, 0.15];
        assert!(t.required_features().contains(&crate::rig::feature::MULTI_PATCH_WHEELS));
    }

    #[test]
    fn the_rig_hash_survives_a_json_round_trip_and_changes_with_one_number() {
        let (tank, _) = box_tank();
        let back: PhysRig = serde_json::from_str(&serde_json::to_string(&tank).unwrap()).unwrap();
        assert_eq!(back, tank, "float_roundtrip makes JSON exact");
        assert_eq!(back.rig_hash(), tank.rig_hash());
        let mut damaged = tank.clone();
        damaged.drivetrain.engine.torque_curve[2].1 *= 0.5;
        assert_ne!(damaged.rig_hash(), tank.rig_hash());
    }

    #[test]
    fn contact_names_list_one_entry_per_patch_and_per_track_sample() {
        let (truck, _) = box_truck();
        let (tank, _) = box_tank();
        assert_eq!(truck.contact_names(), vec!["fl", "fr", "rl", "rr"]);
        assert_eq!(tank.contact_names().len(), 24);
        let mut t = truck;
        t.stations[0].wheel.patches_x_m = vec![-0.15, 0.15];
        assert_eq!(t.contact_names()[..3], ["fl.0".to_string(), "fl.1".to_string(), "fr".to_string()]);
    }

    #[test]
    fn forward_kinematics_swings_the_muzzle_left_when_the_turret_yaws_positive() {
        let (tank, _) = box_tank();
        let q = [1.0, 0.0, 0.0];
        let p = tank.muzzle_pose_in_hull(0, &q).pos;
        // T + R_y(yaw) (G + muzzle offset) with T = (0, 0.6, 0.2), G = (0, 0.35, -1.4), muzzle (0, 0, -4.9).
        let (s, c) = (scalar::sin(1.0), scalar::cos(1.0));
        assert!(
            (p.x - (-6.3 * s)).abs() < 1e-9 && (p.y - 0.95).abs() < 1e-9 && (p.z - (0.2 - 6.3 * c)).abs() < 1e-9,
            "{p:?}"
        );
        assert!(p.x < 0.0, "a positive yaw turns the nose left (toward -X)");
    }

    #[test]
    fn the_composite_mass_and_the_sprung_mass_add_up() {
        let (tank, _) = box_tank();
        let m = tank.composite_mass(&[0.0, 0.0, 0.0]);
        assert!((m.mass_kg - (28_000.0 + 10.0 * 320.0 + 16_000.0)).abs() < 1e-9);
        assert!((tank.sprung_mass_kg() - 44_000.0).abs() < 1e-9);
    }
}
