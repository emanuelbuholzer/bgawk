use crate::{interaction::*, lapis::Lapis};
use avian2d::prelude::*;
use bevy::{prelude::*, sprite::AlphaMode2d};

pub struct ObjectsPlugin;

impl Plugin for ObjectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            spawn
                .run_if(resource_equals(EguiFocused(false)))
                .run_if(resource_equals(Mode::Draw)),
        )
        .add_systems(PhysicsSchedule, attract.in_set(PhysicsStepSet::Last))
        .add_systems(Update, eval_collisions)
        .add_systems(PostUpdate, sync_links)
        .insert_resource(AttractionFactor(0.1));
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Code(pub String);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Links(pub String);

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Sides(pub u32);

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct AttractionFactor(pub f32);

fn spawn(
    mut commands: Commands,
    cursor: Res<CursorInfo>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    settings: Res<DrawSettings>,
    egui_focused: Res<EguiFocused>,
) {
    if !keyboard_input.pressed(KeyCode::Space)
        && mouse_button_input.just_released(MouseButton::Left)
        // avoid spawning when dragging outside of egui
        && !egui_focused.is_changed()
    {
        let r = cursor.f.distance(cursor.i).max(1.0);
        let material = ColorMaterial {
            color: Srgba::from_u8_array(settings.color).into(),
            alpha_mode: AlphaMode2d::Blend,
            ..default()
        };
        let mesh_handle = meshes.add(RegularPolygon::new(1., settings.sides));
        let mat_handle = materials.add(material);
        let layer = 1 << settings.collision_layer;
        let mut e = commands.spawn((
            Mesh2d(mesh_handle),
            MeshMaterial2d(mat_handle),
            settings.rigid_body,
            Links(settings.links.clone()),
            Code(settings.code.clone()),
            Mass(r * 100.),
            AngularInertia(r * 100.),
            CenterOfMass(settings.center_of_mass),
            Collider::regular_polygon(1., settings.sides),
            CollisionLayers::from_bits(layer, layer),
            Restitution::new(settings.restitution),
            LinearDamping(settings.lin_damp),
            AngularDamping(settings.ang_damp),
            Transform {
                translation: cursor.i.extend(0.),
                scale: Vec3::new(r, r, 1.),
                ..default()
            },
            Sides(settings.sides),
        ));
        if settings.sensor {
            e.insert(Sensor);
        }
        if settings.custom_mass {
            e.insert(Mass(settings.mass));
        }
        if settings.custom_inertia {
            e.insert(AngularInertia(settings.inertia));
        }
    }
}

// TODO only work on dynamic objects (optimization)
fn attract(
    layers: Query<(Entity, &CollisionLayers)>,
    mut query: Query<(&Mass, &Position, &mut LinearVelocity)>,
    factor: Res<AttractionFactor>,
) {
    if !factor.0.is_normal() {
        return;
    }
    for (e1, l1) in layers.iter() {
        for (e2, l2) in layers.iter() {
            if l1 == l2 && e1 != e2 {
                let [mut e1, mut e2] = query.many_mut([e1, e2]);
                let m1 = e1.0 .0;
                let m2 = e2.0 .0;
                let p1 = e1.1 .0;
                let p2 = e2.1 .0;
                let r = p1.distance_squared(p2);
                e1.2 .0 += (p2 - p1) * m2 / r * factor.0;
                e2.2 .0 += (p1 - p2) * m1 / r * factor.0;
            }
        }
    }
}

fn eval_collisions(
    mut collision_event_reader: EventReader<Collision>,
    code: Query<&Code>,
    mut lapis: ResMut<Lapis>,
    trans_query: Query<&Transform>,
    lin_velocity_query: Query<&LinearVelocity>,
    ang_velocity_query: Query<&AngularVelocity>,
    mass_query: Query<&Mass>,
    inertia_query: Query<&AngularInertia>,
) {
    for Collision(contacts) in collision_event_reader.read() {
        if contacts.collision_started() {
            for e in [contacts.entity1, contacts.entity2] {
                if let Ok(c) = code.get(e) {
                    if c.0.contains('$') {
                        let trans = trans_query.get(e).unwrap();
                        let lin_v = lin_velocity_query.get(e).unwrap();
                        let x = trans.translation.x;
                        let y = trans.translation.y;
                        let rx = trans.scale.x;
                        let ry = trans.scale.x;
                        let rot = trans.rotation.to_euler(EulerRot::XYZ).2;
                        let vx = lin_v.x;
                        let vy = lin_v.y;
                        let va = ang_velocity_query.get(e).unwrap().0;
                        let mass = mass_query.get(e).unwrap().0;
                        let inertia = inertia_query.get(e).unwrap().0;
                        let code =
                            c.0.replace("$x", &format!("{x}"))
                                .replace("$y", &format!("{y}"))
                                .replace("$rx", &format!("{rx}"))
                                .replace("$ry", &format!("{ry}"))
                                .replace("$rot", &format!("{rot}"))
                                .replace("$vx", &format!("{vx}"))
                                .replace("$vy", &format!("{vy}"))
                                .replace("$va", &format!("{va}"))
                                .replace("$mass", &format!("{mass}"))
                                .replace("$inertia", &format!("{inertia}"));
                        lapis.eval(&code);
                    } else {
                        lapis.eval(&c.0);
                    }
                }
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn sync_links(
    links_query: Query<(Entity, &Links)>,
    mut trans_query: Query<&mut Transform>,
    mut mass_query: Query<&mut Mass>,
    mut lin_velocity_query: Query<&mut LinearVelocity>,
    mut ang_velocity_query: Query<&mut AngularVelocity>,
    mut restitution_query: Query<&mut Restitution>,
    mut lin_damp_query: Query<&mut LinearDamping>,
    mut ang_damp_query: Query<&mut AngularDamping>,
    mut inertia_query: Query<&mut AngularInertia>,
    (material_ids, mut materials, mesh_ids, mut meshes): (
        Query<&MeshMaterial2d<ColorMaterial>>,
        ResMut<Assets<ColorMaterial>>,
        Query<&Mesh2d>,
        ResMut<Assets<Mesh>>,
    ),
    mut collider_query: Query<&mut Collider>,
    sides_query: Query<&Sides>,
    mut cm_query: Query<&mut CenterOfMass>,
    lapis: Res<Lapis>,
) {
    for (e, Links(links)) in links_query.iter() {
        for link in links.lines() {
            // links are in the form "property > var" or "property < var"
            let mut link = link.split_ascii_whitespace();
            let s0 = link.next();
            let s1 = link.next();
            let s2 = link.next();
            let (Some(property), Some(dir), Some(var)) = (s0, s1, s2) else {
                continue;
            };
            if let Some(var) = lapis.smap.get(var) {
                match property {
                    "x" => {
                        let mut trans = trans_query.get_mut(e).unwrap();
                        if dir == "<" {
                            trans.translation.x = var.value();
                        } else if dir == ">" {
                            var.set(trans.translation.x);
                        }
                    }
                    "y" => {
                        let mut trans = trans_query.get_mut(e).unwrap();
                        if dir == "<" {
                            trans.translation.y = var.value();
                        } else if dir == ">" {
                            var.set(trans.translation.y);
                        }
                    }
                    "rx" => {
                        let mut trans = trans_query.get_mut(e).unwrap();
                        if dir == "<" {
                            trans.scale.x = var.value();
                        } else if dir == ">" {
                            var.set(trans.scale.x);
                        }
                    }
                    "ry" => {
                        let mut trans = trans_query.get_mut(e).unwrap();
                        if dir == "<" {
                            trans.scale.y = var.value();
                        } else if dir == ">" {
                            var.set(trans.scale.y);
                        }
                    }
                    "rot" => {
                        let mut trans = trans_query.get_mut(e).unwrap();
                        if dir == "<" {
                            trans.rotation = Quat::from_rotation_z(var.value());
                        } else if dir == ">" {
                            let rot = &mut trans.rotation.to_euler(EulerRot::XYZ).2;
                            var.set(*rot);
                        }
                    }
                    "mass" => {
                        let mut mass = mass_query.get_mut(e).unwrap();
                        if dir == "<" {
                            mass.0 = var.value();
                        } else if dir == ">" {
                            var.set(mass.0);
                        }
                    }
                    "vx" => {
                        let mut velocity = lin_velocity_query.get_mut(e).unwrap();
                        if dir == "<" {
                            velocity.x = var.value();
                        } else if dir == ">" {
                            var.set(velocity.x);
                        }
                    }
                    "vy" => {
                        let mut velocity = lin_velocity_query.get_mut(e).unwrap();
                        if dir == "<" {
                            velocity.y = var.value();
                        } else if dir == ">" {
                            var.set(velocity.y);
                        }
                    }
                    "va" => {
                        let mut velocity = ang_velocity_query.get_mut(e).unwrap();
                        if dir == "<" {
                            velocity.0 = var.value();
                        } else if dir == ">" {
                            var.set(velocity.0);
                        }
                    }
                    "restitution" => {
                        let mut restitution = restitution_query.get_mut(e).unwrap();
                        if dir == "<" {
                            restitution.coefficient = var.value();
                        } else if dir == ">" {
                            var.set(restitution.coefficient);
                        }
                    }
                    "lindamp" => {
                        let mut damp = lin_damp_query.get_mut(e).unwrap();
                        if dir == "<" {
                            damp.0 = var.value();
                        } else if dir == ">" {
                            var.set(damp.0);
                        }
                    }
                    "angdamp" => {
                        let mut damp = ang_damp_query.get_mut(e).unwrap();
                        if dir == "<" {
                            damp.0 = var.value();
                        } else if dir == ">" {
                            var.set(damp.0);
                        }
                    }
                    "inertia" => {
                        let mut inertia = inertia_query.get_mut(e).unwrap();
                        if dir == "<" {
                            inertia.0 = var.value();
                        } else if dir == ">" {
                            var.set(inertia.0);
                        }
                    }
                    "h" => {
                        let mat_id = material_ids.get(e).unwrap();
                        let mat = materials.get_mut(mat_id).unwrap();
                        if dir == "<" {
                            let mut hsla: Hsla = mat.color.into();
                            hsla.hue = var.value();
                            mat.color = hsla.into();
                        } else if dir == ">" {
                            let hsla: Hsla = mat.color.into();
                            var.set(hsla.hue);
                        }
                    }
                    "s" => {
                        let mat_id = material_ids.get(e).unwrap();
                        let mat = materials.get_mut(mat_id).unwrap();
                        if dir == "<" {
                            let mut hsla: Hsla = mat.color.into();
                            hsla.saturation = var.value();
                            mat.color = hsla.into();
                        } else if dir == ">" {
                            let hsla: Hsla = mat.color.into();
                            var.set(hsla.saturation);
                        }
                    }
                    "l" => {
                        let mat_id = material_ids.get(e).unwrap();
                        let mat = materials.get_mut(mat_id).unwrap();
                        if dir == "<" {
                            let mut hsla: Hsla = mat.color.into();
                            hsla.lightness = var.value();
                            mat.color = hsla.into();
                        } else if dir == ">" {
                            let hsla: Hsla = mat.color.into();
                            var.set(hsla.lightness);
                        }
                    }
                    "a" => {
                        let mat_id = material_ids.get(e).unwrap();
                        let mat = materials.get_mut(mat_id).unwrap();
                        if dir == "<" {
                            let mut hsla: Hsla = mat.color.into();
                            hsla.alpha = var.value();
                            mat.color = hsla.into();
                        } else if dir == ">" {
                            let hsla: Hsla = mat.color.into();
                            var.set(hsla.alpha);
                        }
                    }
                    "sides" => {
                        if dir == "<" {
                            let sides = (var.value() as u32).clamp(3, 128);
                            let mesh_id = mesh_ids.get(e).unwrap();
                            let mesh = meshes.get_mut(mesh_id).unwrap();
                            *mesh = RegularPolygon::new(1., sides).into();
                            let mut collider = collider_query.get_mut(e).unwrap();
                            *collider = Collider::regular_polygon(1., sides);
                        } else if dir == ">" {
                            let sides = sides_query.get(e).unwrap();
                            var.set(sides.0 as f32);
                        }
                    }
                    "cmx" => {
                        let mut cm = cm_query.get_mut(e).unwrap();
                        if dir == "<" {
                            cm.0.x = var.value();
                        } else if dir == ">" {
                            var.set(cm.0.x);
                        }
                    }
                    "cmy" => {
                        let mut cm = cm_query.get_mut(e).unwrap();
                        if dir == "<" {
                            cm.0.y = var.value();
                        } else if dir == ">" {
                            var.set(cm.0.y);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
