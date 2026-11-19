use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use shared::{MissionBriefing, MissionDefeat, MissionPhase};

use crate::button_activation::{self, ButtonActivated};
use crate::network::{GameState, NetworkReceiveSet};

#[derive(Component)]
struct Panel;
#[derive(Component)]
struct Open;
#[derive(Component)]
struct Close;
#[derive(Component)]
struct Detail;
#[derive(Component)]
struct Heading;
#[derive(Component)]
struct ProfilePart(usize);
#[derive(Resource, Default)]
pub(crate) struct MissionUi {
    open: bool,
    room: Option<shared::RoomId>,
    phase: Option<MissionPhase>,
}

impl MissionUi {
    pub(crate) fn is_open(&self) -> bool {
        self.open
    }
}

pub struct MissionUiPlugin;
impl Plugin for MissionUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MissionUi>()
            .add_systems(Startup, setup)
            .add_systems(Update, update.after(NetworkReceiveSet));
    }
}

fn profile_canvas_node() -> Node {
    // Same broadside proportions as a cargo in the periscope: 140 m by 24 m.
    // Only width is constrained; layout derives height using the fixed ratio.
    const ASPECT: f32 = 140.0 / 24.0;
    Node {
        width: percent(90),
        max_width: px(60.0 * ASPECT),
        aspect_ratio: Some(ASPECT),
        flex_shrink: 0.0,
        ..default()
    }
}

fn setup(mut commands: Commands) {
    let mut open = commands.spawn((
        Open,
        Button,
        Visibility::Hidden,
        GlobalZIndex(102),
        Node {
            position_type: PositionType::Absolute,
            right: px(76),
            top: px(4),
            min_height: px(44),
            padding: UiRect::all(px(8)),
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.06, 0.22, 0.25)),
    ));
    button_activation::enable(&mut open);
    open.with_child((
        Text::new("MISSION / BRIEFING"),
        TextFont {
            font_size: FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Pickable::IGNORE,
    ));
    commands
        .spawn((
            Panel,
            Visibility::Hidden,
            GlobalZIndex(110),
            ScrollPosition::default(),
            RelativeCursorPosition::default(),
            Node {
                position_type: PositionType::Absolute,
                left: px(12),
                right: px(12),
                top: px(104),
                bottom: px(12),
                padding: UiRect::all(px(14)),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(Color::srgb(0.025, 0.09, 0.12)),
        ))
        .observe(
            |drag: On<Pointer<Drag>>,
             scale: Res<UiScale>,
             mut panel: Single<(&ComputedNode, &mut ScrollPosition), With<Panel>>| {
                let range = (panel.0.content_size.y - panel.0.size.y).max(0.0)
                    * panel.0.inverse_scale_factor;
                panel.1 .0.y = (panel.1 .0.y - drag.delta.y / scale.0).clamp(0.0, range);
            },
        )
        .with_children(|panel| {
            panel.spawn((
                Heading,
                Text::new("NIVEAU 1 // BRIEFING"),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.95, 0.8)),
            ));
            panel
                .spawn((
                    Node {
                        width: percent(100),
                        height: px(76),
                        min_height: px(76),
                        flex_shrink: 0.0,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.55, 0.67, 0.69)),
                    Pickable::IGNORE,
                ))
                .with_children(|frame| {
                    frame
                        .spawn((profile_canvas_node(), Pickable::IGNORE))
                        .with_children(|profile| {
                            for index in 0..8 {
                                profile.spawn((
                                    ProfilePart(index),
                                    Node {
                                        position_type: PositionType::Absolute,
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.02, 0.03, 0.04)),
                                    Pickable::IGNORE,
                                ));
                            }
                        });
                });
            panel.spawn((
                Detail,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            let mut close = panel.spawn((
                Close,
                Button,
                Node {
                    min_height: px(44),
                    flex_shrink: 0.0,
                    padding: UiRect::all(px(8)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.07, 0.30, 0.32)),
            ));
            button_activation::enable(&mut close);
            close.with_child((
                Text::new("FERMER / REVENIR AU POSTE"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Pickable::IGNORE,
            ));
        });
}

fn briefing_text(briefing: MissionBriefing) -> String {
    let zone = briefing.extraction;
    format!(
        "CIBLE PRIORITAIRE\n{}\n\nConvoi : 2 cargos differents + 1 escorte.\nReconnaissez la cible vous-meme au periscope : aucune validation de votre designation. Couler l'autre cargo ne remplit pas l'objectif.\n\nORDRES\nDetruire le cargo cible puis rejoindre l'extraction vivant, meme poursuivi.\nExtraction fixe : X {:.0} / Y {:.0} m, rayon {:.0} m.\n\nDEFAITE\nSous-marin perdu ou cible sortie de la zone d'operation (+/- {:.0} m sur chaque axe). Aucune limite de temps supplementaire.",
        crate::silhouette::description(briefing.target), zone.x, zone.y, zone.radius, briefing.operation_half_extent
    )
}

fn phase_title(phase: Option<MissionPhase>) -> &'static str {
    match phase {
        None => "NIVEAU 1 // BRIEFING",
        Some(MissionPhase::Attack) => "NIVEAU 1 // IDENTIFIER ET ATTAQUER",
        Some(MissionPhase::Extract) => "OBJECTIF DETRUIT // REJOIGNEZ L'EXTRACTION",
        Some(MissionPhase::Victory) => "VICTOIRE // EXTRACTION REUSSIE",
        Some(MissionPhase::Defeat(MissionDefeat::TargetEscaped)) => {
            "DEFAITE // CARGO CIBLE ECHAPPE"
        }
        Some(MissionPhase::Defeat(MissionDefeat::SubmarineLost)) => "DEFAITE // SOUS-MARIN PERDU",
    }
}

fn update(
    state: Res<GameState>,
    mut ui: ResMut<MissionUi>,
    mut clicks: MessageReader<ButtonActivated>,
    mut wheel: MessageReader<MouseWheel>,
    mut panel: Single<
        (
            &mut Visibility,
            &ComputedNode,
            &mut ScrollPosition,
            &RelativeCursorPosition,
        ),
        With<Panel>,
    >,
    mut open: Single<(Entity, &mut Visibility), (With<Open>, Without<Panel>)>,
    close: Single<Entity, With<Close>>,
    mut heading: Single<&mut Text, (With<Heading>, Without<Detail>)>,
    mut detail: Single<&mut Text, (With<Detail>, Without<Heading>)>,
    mut parts: Query<(&ProfilePart, &mut Node)>,
) {
    let briefing = state
        .mission
        .map(|m| m.briefing)
        .or_else(|| state.lobby.as_ref().map(|l| l.briefing));
    *open.1 = if briefing.is_some() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    let phase = state.mission.map(|m| m.phase);
    let room = state.lobby.as_ref().map(|l| l.room_id.clone());
    if room != ui.room || phase != ui.phase {
        // Open on arrival, objective completion and mission end, not on each snapshot.
        ui.open = briefing.is_some();
        ui.room = room;
        ui.phase = phase;
        panel.2 .0.y = 0.0;
    }
    for click in clicks.read() {
        if click.0 == open.0 && briefing.is_some() {
            ui.open = !ui.open;
        }
        if click.0 == *close {
            ui.open = false;
        }
    }
    *panel.0 = if ui.open && briefing.is_some() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for event in wheel.read() {
        if ui.open && panel.3.cursor_over() {
            let delta = match event.unit {
                MouseScrollUnit::Line => event.y * 44.0,
                MouseScrollUnit::Pixel => event.y,
            };
            let range =
                (panel.1.content_size.y - panel.1.size.y).max(0.0) * panel.1.inverse_scale_factor;
            panel.2 .0.y = (panel.2 .0.y - delta).clamp(0.0, range);
        }
    }
    let Some(briefing) = briefing else {
        return;
    };
    heading.0 = phase_title(phase).into();
    detail.0 = briefing_text(briefing);
    if let Some(mission) = state.mission {
        if mission.phase.finished() {
            let own = state
                .submarine
                .as_ref()
                .map(|s| format!("\nCoque restante : {:.0}%", s.common.damage.hull_integrity))
                .unwrap_or_default();
            detail.0 = format!("{}\nDuree : {:.0} min {:.0} s{}\n\nLa simulation est arretee. Pour une nouvelle mission, rechargez la page et creez une nouvelle salle.\n\n{}", phase_title(Some(mission.phase)), (mission.elapsed_seconds / 60.0).floor(), mission.elapsed_seconds % 60.0, own, briefing_text(briefing));
        }
    }
    for (part, mut node) in &mut parts {
        let (x, y, w, h) = crate::silhouette::parts(briefing.target)[part.0];
        node.left = percent(x * 100.0);
        node.bottom = percent(y * 100.0);
        node.width = percent(w * 100.0);
        node.height = percent(h * 100.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn briefing_profile_constrains_only_width_and_preserves_cargo_aspect() {
        let node = profile_canvas_node();
        assert_eq!(node.width, percent(90));
        assert_eq!(node.height, Val::Auto);
        assert_eq!(node.aspect_ratio, Some(140.0 / 24.0));
        assert_eq!(node.max_width, px(350.0));
        assert_eq!(node.flex_shrink, 0.0);
    }

    #[test]
    fn briefing_names_only_profile_and_public_geometry() {
        let text = briefing_text(MissionBriefing::level_one(0));
        assert!(text.contains("UNE cheminee"));
        assert!(text.contains("500"));
        assert!(phase_title(Some(MissionPhase::Extract)).contains("OBJECTIF DETRUIT"));
        assert!(
            phase_title(Some(MissionPhase::Defeat(MissionDefeat::TargetEscaped)))
                .contains("ECHAPPE")
        );
    }
}
