use crate::{interaction::*, lapis::*, objects::*};
use avian2d::prelude::*;
use bevy::{
    app::{App, Plugin, Update},
    core_pipeline::{
        bloom::{Bloom, BloomCompositeMode},
        tonemapping::Tonemapping,
    },
    prelude::{
        ClearColor, ColorToPacked, GizmoConfigStore, MonitorSelection, Query, Res, ResMut,
        Resource, Srgba, Time, Virtual, With,
    },
    window::WindowMode,
};
use bevy_egui::{EguiContexts, EguiPlugin};
use egui::*;
use egui_extras::syntax_highlighting::*;
use std::sync::Arc;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .init_resource::<InsertComponents>()
            .insert_resource(ScaleFactor(1.))
            .init_resource::<UpdateCode>()
            .add_systems(Update, egui_ui);
    }
}

#[derive(Resource, Default)]
struct UpdateCode(String);

#[derive(Resource, Default)]
struct InsertComponents {
    links: String,
    code: (String, String),
}

#[derive(Resource)]
pub struct ScaleFactor(pub f32);

fn egui_ui(
    mut contexts: EguiContexts,
    mut lapis: Lapis,
    mut draw: ResMut<DrawSettings>,
    mut gravity: ResMut<Gravity>,
    mut selected: Query<(&mut Code, &mut Links), With<Selected>>,
    mut update_code: ResMut<UpdateCode>,
    mut mode: ResMut<Mode>,
    mut attraction_factor: ResMut<AttractionFactor>,
    mut joint: ResMut<JointSettings>,
    mut time: ResMut<Time<Virtual>>,
    mut insert: ResMut<InsertComponents>,
    cursor: Res<CursorInfo>,
    mut config_store: ResMut<GizmoConfigStore>,
    (mut scale_factor, mut win, mut clear_color): (
        ResMut<ScaleFactor>,
        Query<&mut bevy::prelude::Window>,
        ResMut<ClearColor>,
    ),
    (mut bloom, mut tonemapping): (Query<&mut Bloom>, Query<&mut Tonemapping>),
) {
    let ctx = contexts.ctx_mut();
    let theme = CodeTheme::from_memory(ctx, &ctx.style());
    let mut layouter = |ui: &Ui, string: &str, wrap_width: f32| {
        let mut layout_job = highlight(ui.ctx(), ui.style(), &theme, string, "rs");
        layout_job.wrap.max_width = wrap_width;
        ui.fonts(|f| f.layout_job(layout_job))
    };
    if lapis.data.keys_active {
        if lapis.data.quiet {
            for (shortcut, code) in lapis.data.keys.clone() {
                if ctx.input_mut(|i| i.consume_shortcut(&shortcut)) {
                    lapis.quiet_eval(&code);
                }
            }
        } else {
            for (shortcut, code) in lapis.data.keys.clone() {
                if ctx.input_mut(|i| i.consume_shortcut(&shortcut)) {
                    lapis.eval(&code);
                }
            }
        }
    }
    Window::new("mode").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut *mode, Mode::Edit, "Edit")
                .on_hover_text("ctrl+1");
            ui.selectable_value(&mut *mode, Mode::Draw, "Draw")
                .on_hover_text("ctrl+2");
            ui.selectable_value(&mut *mode, Mode::Joint, "Joint")
                .on_hover_text("ctrl+3");
        });
        ui.separator();
        if *mode == Mode::Draw {
            ui.horizontal(|ui| {
                ui.label("rigid body");
                ui.selectable_value(&mut draw.rigid_body, RigidBody::Static, "Static");
                ui.selectable_value(&mut draw.rigid_body, RigidBody::Dynamic, "Dynamic");
            });
            ui.horizontal(|ui| {
                ui.label("collision layer");
                ui.add(DragValue::new(&mut draw.collision_layer).range(0..=31));
            });
            ui.horizontal(|ui| {
                ui.label("sides");
                ui.add(DragValue::new(&mut draw.sides).range(3..=512));
            });
            ui.horizontal(|ui| {
                ui.label("color");
                ui.color_edit_button_srgba_unmultiplied(&mut draw.color);
            });
            ui.horizontal(|ui| {
                ui.label("tail");
                ui.add(DragValue::new(&mut draw.tail).range(0..=36000))
                    .on_hover_text("tail length in points");
            });
            ui.horizontal(|ui| {
                ui.toggle_value(&mut draw.custom_mass, "custom mass?")
                    .on_hover_text("if not selected, mass = radius ^ 3");
                ui.add_enabled(draw.custom_mass, DragValue::new(&mut draw.mass));
            });
            ui.horizontal(|ui| {
                ui.toggle_value(&mut draw.custom_inertia, "custom inertia?")
                    .on_hover_text("if not selected, inertia = radius ^ 3");
                ui.add_enabled(draw.custom_inertia, DragValue::new(&mut draw.inertia));
            });
            ui.horizontal(|ui| {
                ui.label("center of mass");
                ui.add(DragValue::new(&mut draw.center_of_mass.x).speed(0.1));
                ui.add(DragValue::new(&mut draw.center_of_mass.y).speed(0.1));
            });
            ui.horizontal(|ui| {
                ui.label("friction");
                ui.add(DragValue::new(&mut draw.friction).speed(0.01));
            });
            ui.horizontal(|ui| {
                ui.label("restitution");
                ui.add(DragValue::new(&mut draw.restitution).speed(0.01));
            });
            ui.toggle_value(&mut draw.sensor, "sensor?")
                .on_hover_text("allows other bodies to pass through");
            ui.horizontal(|ui| {
                ui.label("linear damping");
                ui.add(DragValue::new(&mut draw.lin_damp).speed(0.01));
            });
            ui.horizontal(|ui| {
                ui.label("angular damping");
                ui.add(DragValue::new(&mut draw.ang_damp).speed(0.01));
            });
            links_line(ui, &mut draw.links);
            code_line(ui, &mut draw.code.0, &mut layouter, "on collision start");
            code_line(ui, &mut draw.code.1, &mut layouter, "on collision end");
        } else if *mode == Mode::Edit {
            if time.is_paused() {
                if ui.button("resume").clicked() {
                    time.unpause();
                }
            } else if ui.button("pause").clicked() {
                time.pause();
            }
            ui.horizontal(|ui| {
                ui.label("gravity");
                ui.add(DragValue::new(&mut gravity.0.x));
                ui.add(DragValue::new(&mut gravity.0.y));
            });
            ui.horizontal(|ui| {
                ui.label("attraction");
                ui.add(DragValue::new(&mut attraction_factor.0).speed(0.01))
                    .on_hover_text("how much objects gravitate towards each other");
            });
            ui.collapsing("ui settings", |ui| {
                ui.horizontal(|ui| {
                    ui.label("scale factor");
                    let factor = ui.add(
                        DragValue::new(&mut scale_factor.0)
                            .range(0.5..=4.)
                            .speed(0.1),
                    );
                    if factor.changed() {
                        win.single_mut().resolution.set_scale_factor(scale_factor.0);
                    }
                });
                ui.horizontal(|ui| {
                    let fullscreen = WindowMode::Fullscreen(MonitorSelection::Current);
                    let windowed = WindowMode::Windowed;
                    ui.label("win mode");
                    ui.selectable_value(&mut win.single_mut().mode, fullscreen, "fullscreen");
                    ui.selectable_value(&mut win.single_mut().mode, windowed, "windowed");
                });
                ui.horizontal(|ui| {
                    ui.label("clear color");
                    let mut tmp = clear_color.0.to_srgba().to_u8_array();
                    ui.color_edit_button_srgba_unmultiplied(&mut tmp);
                    clear_color.0 = Srgba::from_u8_array(tmp).into();
                });
                ui.collapsing("bloom", |ui| {
                    let bloom = &mut bloom.single_mut();
                    ui.horizontal(|ui| {
                        ui.label("intensity");
                        ui.add(DragValue::new(&mut bloom.intensity).speed(0.1));
                    });
                    ui.horizontal(|ui| {
                        ui.label("low freq boost");
                        ui.add(DragValue::new(&mut bloom.low_frequency_boost).speed(0.1));
                    });
                    ui.horizontal(|ui| {
                        ui.label("lf boost curvature");
                        ui.add(DragValue::new(&mut bloom.low_frequency_boost_curvature).speed(0.1));
                    });
                    ui.horizontal(|ui| {
                        ui.label("hight pass freq");
                        ui.add(DragValue::new(&mut bloom.high_pass_frequency).speed(0.1));
                    });
                    ui.horizontal(|ui| {
                        ui.label("prefilter threshold");
                        ui.add(DragValue::new(&mut bloom.prefilter.threshold).speed(0.1));
                    });
                    ui.horizontal(|ui| {
                        ui.label("threshold softness");
                        ui.add(DragValue::new(&mut bloom.prefilter.threshold_softness).speed(0.1));
                    });
                    ui.horizontal(|ui| {
                        ui.label("composite");
                        let conserving = BloomCompositeMode::EnergyConserving;
                        let additive = BloomCompositeMode::Additive;
                        ui.selectable_value(
                            &mut bloom.composite_mode,
                            conserving,
                            "energy conserving",
                        );
                        ui.selectable_value(&mut bloom.composite_mode, additive, "additive");
                    });
                    ui.horizontal(|ui| {
                        ui.label("max mip dimension");
                        ui.add(DragValue::new(&mut bloom.max_mip_dimension).range(1..=1024));
                    });
                    ui.horizontal(|ui| {
                        ui.label("uv offset");
                        ui.add(DragValue::new(&mut bloom.uv_offset).speed(0.1));
                    });
                });
                ui.horizontal(|ui| {
                    ui.label("tonemapping");
                    egui::ComboBox::from_label("")
                        .selected_text(format!("{:?}", tonemapping.single()))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut *tonemapping.single_mut(),
                                Tonemapping::None,
                                "None",
                            );
                            ui.selectable_value(
                                &mut *tonemapping.single_mut(),
                                Tonemapping::Reinhard,
                                "Reinhard",
                            );
                            ui.selectable_value(
                                &mut *tonemapping.single_mut(),
                                Tonemapping::ReinhardLuminance,
                                "ReinhardLuminance",
                            );
                            ui.selectable_value(
                                &mut *tonemapping.single_mut(),
                                Tonemapping::AcesFitted,
                                "AcesFitted",
                            );
                            ui.selectable_value(
                                &mut *tonemapping.single_mut(),
                                Tonemapping::AgX,
                                "AgX",
                            );
                            ui.selectable_value(
                                &mut *tonemapping.single_mut(),
                                Tonemapping::SomewhatBoringDisplayTransform,
                                "SomewhatBoringDisplayTransform",
                            );
                            ui.selectable_value(
                                &mut *tonemapping.single_mut(),
                                Tonemapping::TonyMcMapface,
                                "TonyMcMapface",
                            );
                            ui.selectable_value(
                                &mut *tonemapping.single_mut(),
                                Tonemapping::BlenderFilmic,
                                "BlenderFilmic",
                            );
                        });
                });
            });
            ui.separator();
            let n = selected.iter().len();
            ui.label(format!("selected: {}", n));
            match n {
                0 => {}
                1 => {
                    let (mut code, mut links) = selected.single_mut();
                    links_line(ui, &mut links.0);
                    code_line(ui, &mut code.0, &mut layouter, "on collision start");
                    code_line(ui, &mut code.1, &mut layouter, "on collision end");
                }
                _ => {
                    links_line(ui, &mut insert.links);
                    code_line(ui, &mut insert.code.0, &mut layouter, "on collision start");
                    code_line(ui, &mut insert.code.1, &mut layouter, "on collision end");
                    if ui.button("apply to selected").clicked() {
                        for (mut code, mut links) in selected.iter_mut() {
                            code.0 = insert.code.0.clone();
                            code.1 = insert.code.1.clone();
                            links.0 = insert.links.clone();
                        }
                    }
                }
            }
        } else if *mode == Mode::Joint {
            ui.horizontal(|ui| {
                ui.label("type");
                ui.selectable_value(&mut joint.joint_type, JointType::Distance, "Distance");
                ui.selectable_value(&mut joint.joint_type, JointType::Prismatic, "Prismatic");
                ui.selectable_value(&mut joint.joint_type, JointType::Revolute, "Revolute");
                ui.selectable_value(&mut joint.joint_type, JointType::Fixed, "Fixed");
            });
            ui.horizontal(|ui| {
                ui.label("compliance");
                ui.add(
                    DragValue::new(&mut joint.compliance)
                        .range(0.0..=f32::INFINITY)
                        .speed(0.001),
                );
            });
            ui.toggle_value(&mut joint.custom_anchors, "custom anchors?");
            if joint.custom_anchors {
                ui.horizontal(|ui| {
                    ui.label("local anchor 1");
                    ui.add(DragValue::new(&mut joint.local_anchor_1.x).speed(0.01));
                    ui.add(DragValue::new(&mut joint.local_anchor_1.y).speed(0.01));
                });
                ui.horizontal(|ui| {
                    ui.label("local anchor 2");
                    ui.add(DragValue::new(&mut joint.local_anchor_2.x).speed(0.01));
                    ui.add(DragValue::new(&mut joint.local_anchor_2.y).speed(0.01));
                });
            }
            match joint.joint_type {
                JointType::Distance => {
                    ui.horizontal(|ui| {
                        ui.label("limits");
                        ui.add(
                            DragValue::new(&mut joint.dist_limits.0)
                                .range(0.0..=f32::INFINITY)
                                .speed(0.01),
                        );
                        ui.add(
                            DragValue::new(&mut joint.dist_limits.1)
                                .range(0.0..=f32::INFINITY)
                                .speed(0.01),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("rest length");
                        ui.add(
                            DragValue::new(&mut joint.dist_rest)
                                .range(0.0..=f32::INFINITY)
                                .speed(0.01),
                        );
                    });
                }
                JointType::Prismatic => {
                    ui.horizontal(|ui| {
                        ui.label("limits");
                        ui.add(DragValue::new(&mut joint.prismatic_limits.0).speed(0.01));
                        ui.add(DragValue::new(&mut joint.prismatic_limits.1).speed(0.01));
                    });
                    ui.horizontal(|ui| {
                        ui.label("free axis");
                        ui.add(DragValue::new(&mut joint.prismatic_axis.x).speed(0.01));
                        ui.add(DragValue::new(&mut joint.prismatic_axis.y).speed(0.01));
                    });
                }
                JointType::Revolute => {
                    ui.horizontal(|ui| {
                        ui.label("limits");
                        ui.add(DragValue::new(&mut joint.angle_limits.0).speed(0.01));
                        ui.add(DragValue::new(&mut joint.angle_limits.1).speed(0.01));
                    });
                }
                _ => {}
            }
        }
    });
    let res = &win.single().resolution;
    let (w, h) = (res.width(), res.height());
    Window::new("lapis output")
        .pivot(Align2::RIGHT_TOP)
        .default_pos([w - 15., 15.])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.toggle_value(&mut lapis.data.quiet, "quiet?")
                    .on_hover_text("don't log collision/keybinding evaluation");
                ui.toggle_value(&mut lapis.data.keys_active, "keys?")
                    .on_hover_text("enable keybindings");
            });
            ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut lapis.data.buffer)
                        .code_editor()
                        .desired_rows(1)
                        .desired_width(f32::INFINITY)
                        .layouter(&mut layouter),
                );
            });
        });
    Window::new("lapis input")
        .pivot(Align2::RIGHT_BOTTOM)
        .default_pos([w - 15., h - 15.])
        .show(ctx, |ui| {
            ui.collapsing("update code", |ui| {
                ui.add(
                    TextEdit::multiline(&mut update_code.0)
                        .hint_text("code here is quietly evaluated every frame")
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .layouter(&mut layouter),
                );
            });
            lapis.quiet_eval(&update_code.0);
            ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                        let execute = ui.button("e");
                        let input_focused = ui
                            .add(
                                TextEdit::multiline(&mut lapis.data.input)
                                    .hint_text("type code then press ctrl+enter")
                                    .code_editor()
                                    .desired_rows(5)
                                    .desired_width(f32::INFINITY)
                                    .layouter(&mut layouter),
                            )
                            .has_focus();
                        let shortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Enter);
                        if input_focused && ctx.input_mut(|i| i.consume_shortcut(&shortcut))
                            || execute.clicked()
                        {
                            lapis.eval_input();
                        }
                    });
                });
            });
        });
    Window::new("info")
        .default_open(false)
        .pivot(Align2::LEFT_BOTTOM)
        .default_pos([15., h - 15.])
        .show(ctx, |ui| {
            let (conf, _) = config_store.config_mut::<PhysicsGizmos>();
            ui.toggle_value(&mut conf.enabled, "debug?")
                .on_hover_text("show gizmos for objects/joints");
            ui.label(format!("i: ({}, {})", cursor.i.x, cursor.i.y));
            ui.label(format!("f: ({}, {})", cursor.f.x, cursor.f.y));
            ui.label(format!("distance: {}", cursor.i.distance(cursor.f)));
            ui.horizontal(|ui| {
                if ui.button("help").clicked() {
                    lapis.data.help = !lapis.data.help;
                }
                if ui.button("about").clicked() {
                    lapis.data.about = !lapis.data.about;
                }
            });
        });
    Window::new("about")
        .open(&mut lapis.data.about)
        .show(ctx, about_window_function);
    Window::new("help")
        .open(&mut lapis.data.help)
        .show(ctx, help_window_function);
}

fn links_line(ui: &mut Ui, buffer: &mut String) {
    ui.horizontal(|ui| {
        ui.label("links");
        ui.add(
            TextEdit::multiline(buffer)
                .code_editor()
                .desired_rows(1)
                .desired_width(f32::INFINITY),
        )
        .on_hover_text(LINKS_TOOLTIP);
    });
}

fn code_line(
    ui: &mut Ui,
    buffer: &mut String,
    layouter: &mut dyn FnMut(&Ui, &str, f32) -> Arc<Galley>,
    hint: &str,
) {
    ui.horizontal(|ui| {
        ui.label("code");
        ui.add(
            TextEdit::multiline(buffer)
                .hint_text(hint)
                .code_editor()
                .desired_rows(1)
                .desired_width(f32::INFINITY)
                .layouter(layouter),
        )
        .on_hover_text(CODE_TOOLTIP);
    });
}

fn about_window_function(ui: &mut Ui) {
    ui.label("this is a toy for playing with physics and sound");
    ui.label("lapis is a FunDSP interpreter");
    ui.horizontal(|ui| {
        ui.label("FunDSP:");
        ui.hyperlink_to(
            "github.com/SamiPerttu/fundsp",
            "https://github.com/SamiPerttu/fundsp/",
        );
    });
    ui.horizontal(|ui| {
        ui.label("FunDSP doc:");
        ui.hyperlink_to(
            "docs.rs/fundsp/latest/fundsp",
            "https://docs.rs/fundsp/latest/fundsp/",
        );
    });
    ui.horizontal(|ui| {
        ui.label("lapis:");
        ui.hyperlink_to(
            "github.com/tomara-x/lapis",
            "https://github.com/tomara-x/lapis/",
        );
    });
    ui.horizontal(|ui| {
        ui.label("lapis mirror:");
        ui.hyperlink_to(
            "codeberg.org/tomara-x/lapis",
            "https://codeberg.org/tomara-x/lapis/",
        );
    });
    ui.horizontal(|ui| {
        ui.label("repo:");
        ui.hyperlink_to(
            "github.com/tomara-x/bgawk",
            "https://github.com/tomara-x/bgawk/",
        );
    });
    ui.horizontal(|ui| {
        ui.label("mirror:");
        ui.hyperlink_to(
            "codeberg.org/tomara-x/bgawk",
            "https://codeberg.org/tomara-x/bgawk/",
        );
    });
    ui.label("an amy universe piece");
    ui.label("courtesy of the alphabet mafia");
    ui.small("made in africa");
}

fn help_window_function(ui: &mut Ui) {
    ScrollArea::vertical().show(ui, |ui| {
        ui.label(
            "- hold space and drag/scroll to pan/zoom the camera
(or to cancel creating an object)
- hold the right mouse button while one object is selected
  to track it with the camera
- in edit mode:
    - press ctrl+a to select all objects
    - when selecting objects, hold shift to add to selection
      or hold ctrl to remove from selection
    - press delete to delete selected objects
    - press shift+delete to delete any joints connected to
      selected objects
    - if you don't need objects to gravitate towards each
      other set the attraction to zero. this will disable
      that system allowing much better performance",
        );
        ui.strong("entities through lapis:");
        ui.collapsing("entity creation/deletion", |ui| {
            ui.label("- to spawn an object with values from draw settings:");
            ui.code("spawn(r); // r is radius");
            ui.label("- to create a joint between 2 points:");
            ui.label("(those points must intersect 2 objects)");
            ui.code("joint(x1, y1, x2, y2);");
            ui.label("- both of these functions return an id which can be assigned");
            ui.code(
                "let e1 = spawn(10);
let e2 = spawn(20).x(200);
let joint = joint(0,0,200,0);",
            );
            ui.label("- to despawn an entity (object or joint)");
            ui.code("entity.despawn();");
        });
        ui.collapsing("PLACEHOLDER", |ui| {
            ui.label("you can create a temporary placeholder entity");
            ui.code("let entity = Entity::PLACEHOLDER;");
            ui.label("methods applied to this affect all selected objects");
        });
        ui.collapsing("object methods", |ui| {
            ui.label("these methods can be called on objects");
            ui.label("all of which return the entity id");
            ui.horizontal(|ui| {
                ui.label("so ");
                ui.code("entity.x(f).y(f).mass(f);");
                ui.label("is valid");
            });
            ui.monospace(
                "- entity.x(f)
- entity.y(f)
- entity.rx(f) // x radius
- entity.ry(f)
- entity.rot(f) // rotation
- entity.mass(f)
- entity.vx(f) // x velocity
- entity.vy(f)
- entity.va(f) // angular velocity
- entity.restitution(f)
- entity.lindamp(f) // linear damping
- entity.angdamp(f) // angular damping
- entity.inertia(f)
- entity.h(f)  // hue
- entity.s(f)  // saturation
- entity.l(f)  // lightness 
- entity.a(f)  // alpha
- entity.sides(f)
- entity.cmx(f)  // x center of mass
- entity.cmy(f)
- entity.friction(f)
- entity.tail(f)  // tail length (in points)
- entity.layer(f)  // collision layer

- entity.dynamic(bool) // dynamic or static
- entity.sensor(bool) // sensors don't collide

// str must be \"in quotes\"
- entity.links(str)  // links text
- entity.code_i(str) // collision start code
- entity.code_f(str) // collision end

// despawns joints connected to this object
- entity.disjoint()
",
            );
        });
        ui.collapsing("object fields", |ui| {
            ui.label("you can get properties of objects");
            ui.label("fun stuff in collision code ;)");
            ui.monospace(
                "- entity.x
- entity.y
- entity.rx
- entity.ry
- entity.rot
- entity.mass
- entity.vx
- entity.vy
- entity.va
- entity.restitution
- entity.lindamp
- entity.angdamp
- entity.inertia
- entity.h
- entity.s
- entity.l
- entity.a
- entity.sides
- entity.cmx
- entity.cmy
- entity.friction
- entity.tail
- entity.layer
- entity.dynamic // bool
- entity.sensor // bool",
            );
        });
        ui.collapsing("joint methods", |ui| {
            ui.monospace(
                "// 0 = fixed, 1 = distance,
// 2 = prismatic, 3 = revolute
- entity.joint_type(f)

- entity.compliance(f)
- entity.anchor1(f, f)
- entity.anchor2(f, f)

// distance / free axis / angle limits
- entity.limits(f, f)

// distance joint rest length
- entity.rest(f)

// prismatic joint free axis
- entity.free_axis(f, f)",
            );
        });
        ui.collapsing("joint fields", |ui| {
            ui.monospace(
                "- entity.joint_type //same numbers as method
- entity.compliance
- entity.anchor1x
- entity.anchor1y
- entity.anchor2x
- entity.anchor2y
- entity.min
- entity.max
- entity.rest
- entity.axis_x
- entity.axis_y
",
            );
        });
        ui.collapsing("entity/float conversion", |ui| {
            ui.label("you can convert an entity to 2 floats");
            ui.code("let floats = entity.to_floats();");
            ui.label("and convert a [f, f] back to an entity");
            ui.code("let same = Entity::from_floats(floats);");
            ui.label("this is useful for storing a collection of entities");
            ui.label("(too lazy to implement arrays for entities :p)");
        });
        ui.label("");
        ui.horizontal(|ui| {
            ui.label("see the");
            ui.hyperlink_to("FunDSP readme", "https://github.com/SamiPerttu/fundsp/");
            ui.label("and");
            ui.hyperlink_to("documentation", "https://docs.rs/fundsp/latest/fundsp/");
        });
        ui.horizontal(|ui| {
            ui.label("and the");
            ui.hyperlink_to("lapis readme", "https://github.com/tomara-x/lapis/");
            ui.label("for more info about how they work");
        });
    });
}

const LINKS_TOOLTIP: &str = "link a property of this entity to a shared var\n
every line should follow the form:
property > variable
to set the variable to the property's value
or
property < variable
to set the property to the variable's value\n
note: float expressions also work in assignment
e.g. \"mass < 5\", \"y < sin(s.value())\", or \"rot = PI*3\"
(no spaces)\n
properties list:
x
y
rx (x radius)
ry
rot (rotation)
mass
vx (x velocity)
vy
va (angular velocity)
vm (velocity magnitude) (polar)
vp (velocity phase) (polar)
restitution
lindamp (linear damping)
angdamp (angular damping)
inertia
h (hue)
s (saturation)
l (lightness)
a (alpha)
sides
cmx (center of mass x)
cmy (center of mass y)
friction
tail (tail length in points)";

const CODE_TOOLTIP: &str = "evaluated when this object starts/stops colliding with another\n
these placeholders will be substituted:
$id for this entity's id
$other for the other entity's id";
