from pathlib import Path

path = Path("crates/rarog-paint/src/lib.rs")
s = path.read_text()

start = s.find("impl DamageRegion {\n")
end = s.find("fn transform_rect(", start)
if start < 0 or end < 0:
    raise SystemExit("damage calculation block markers missing")

replacement = '''#[derive(Clone, Copy, Debug, PartialEq)]
enum EffectivePaintVisual {
    Solid(Color),
    Image {
        destination: Rect,
        image: ImageResourceRef,
        opacity: Opacity,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct EffectivePaintItem {
    ordinal: usize,
    bounds: Option<Rect>,
    visual: EffectivePaintVisual,
}

fn effective_indexed_paint(list: &DisplayList) -> BTreeMap<DisplayItemId, EffectivePaintItem> {
    let mut output = BTreeMap::new();
    let mut transforms = Vec::new();
    let mut clips: Vec<Option<Rect>> = vec![None];
    let mut opacities = vec![Opacity::ONE];
    let mut ordinal = 0usize;

    for (id, command) in list
        .command_ids
        .iter()
        .copied()
        .zip(list.commands.iter().copied())
    {
        match command {
            DisplayCommand::FillRect { rect, color }
            | DisplayCommand::TextPlaceholder { rect, color } => {
                let destination = transform_rect(rect, &transforms);
                let bounds = match *clips.last().expect("clip state") {
                    Some(clip) => intersection(destination, clip),
                    None => Some(destination),
                };
                let color = apply_opacity(color, *opacities.last().expect("opacity state"));
                output.insert(
                    id,
                    EffectivePaintItem {
                        ordinal,
                        bounds,
                        visual: EffectivePaintVisual::Solid(color),
                    },
                );
                ordinal = ordinal.saturating_add(1);
            }
            DisplayCommand::DrawImage { rect, image } => {
                let destination = transform_rect(rect, &transforms);
                let bounds = match *clips.last().expect("clip state") {
                    Some(clip) => intersection(destination, clip),
                    None => Some(destination),
                };
                output.insert(
                    id,
                    EffectivePaintItem {
                        ordinal,
                        bounds,
                        visual: EffectivePaintVisual::Image {
                            destination,
                            image,
                            opacity: *opacities.last().expect("opacity state"),
                        },
                    },
                );
                ordinal = ordinal.saturating_add(1);
            }
            DisplayCommand::PushClip { rect } => {
                let rect = transform_rect(rect, &transforms);
                let clip = match *clips.last().expect("clip state") {
                    Some(current) => Some(
                        intersection(current, rect)
                            .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0)),
                    ),
                    None => Some(rect),
                };
                clips.push(clip);
            }
            DisplayCommand::PopClip => {
                if clips.len() > 1 {
                    clips.pop();
                }
            }
            DisplayCommand::PushStackingContext { .. } | DisplayCommand::PopStackingContext => {}
            DisplayCommand::PushTransform { transform } => transforms.push(transform),
            DisplayCommand::PopTransform => {
                transforms.pop();
            }
            DisplayCommand::PushOpacity { opacity } => {
                let current = *opacities.last().expect("opacity state");
                opacities.push(current.multiply(opacity));
            }
            DisplayCommand::PopOpacity => {
                if opacities.len() > 1 {
                    opacities.pop();
                }
            }
        }
    }

    output
}

impl DamageRegion {
    pub fn between(previous: Option<&DisplayList>, current: &DisplayList) -> Self {
        let current_items = effective_indexed_paint(current);
        let Some(previous) = previous else {
            let mut damage = Self::default();
            for bounds in current_items.values().filter_map(|item| item.bounds) {
                damage.push(bounds);
            }
            return damage;
        };

        let previous_items = effective_indexed_paint(previous);
        let ids = previous_items
            .keys()
            .chain(current_items.keys())
            .copied()
            .collect::<BTreeSet<_>>();
        let mut damage = Self::default();

        for id in ids {
            let before = previous_items.get(&id).copied();
            let after = current_items.get(&id).copied();
            if before == after {
                continue;
            }
            if let Some(bounds) = before.and_then(|item| item.bounds) {
                damage.push(bounds);
            }
            if let Some(bounds) = after.and_then(|item| item.bounds) {
                damage.push(bounds);
            }
        }

        damage
    }

    fn push(&mut self, rect: Rect) {
        if rect.size.width <= 0.0 || rect.size.height <= 0.0 {
            return;
        }
        if !self.rects.contains(&rect) {
            self.rects.push(rect);
        }
    }
}

'''
s = s[:start] + replacement + s[end:]

test_marker = '''    #[test]
    fn structural_damage_uses_transformed_paint_bounds() {
'''
tests = '''    #[test]
    fn structural_damage_ignores_unchanged_paint_outside_changed_transform_scope() {
        let make = |translation: f32| DisplayList {
            command_ids: vec![
                DisplayItemId::test(1),
                DisplayItemId::test(2),
                DisplayItemId::test(3),
                DisplayItemId::test(4),
            ],
            commands: vec![
                DisplayCommand::FillRect {
                    rect: Rect::new(0.0, 0.0, 2.0, 2.0),
                    color: Color::BLACK,
                },
                DisplayCommand::PushTransform {
                    transform: Transform2D::translation(translation, 0.0),
                },
                DisplayCommand::FillRect {
                    rect: Rect::new(10.0, 0.0, 2.0, 2.0),
                    color: Color::rgb(255, 0, 0),
                },
                DisplayCommand::PopTransform,
            ],
        };
        let before = make(0.0);
        let after = make(4.0);
        let damage = DamageRegion::between(Some(&before), &after);

        assert!(!damage.rects.contains(&Rect::new(0.0, 0.0, 2.0, 2.0)));
        assert!(damage.rects.contains(&Rect::new(10.0, 0.0, 2.0, 2.0)));
        assert!(damage.rects.contains(&Rect::new(14.0, 0.0, 2.0, 2.0)));
        assert_eq!(damage.rects.len(), 2);
    }

    #[test]
    fn structural_damage_tracks_effective_opacity_changes() {
        let make = |opacity: f32| DisplayList {
            command_ids: vec![
                DisplayItemId::test(1),
                DisplayItemId::test(2),
                DisplayItemId::test(3),
            ],
            commands: vec![
                DisplayCommand::PushOpacity {
                    opacity: Opacity::new(opacity).unwrap(),
                },
                DisplayCommand::FillRect {
                    rect: Rect::new(3.0, 2.0, 4.0, 5.0),
                    color: Color::BLACK,
                },
                DisplayCommand::PopOpacity,
            ],
        };
        let damage = DamageRegion::between(Some(&make(0.25)), &make(0.75));

        assert_eq!(damage.rects, vec![Rect::new(3.0, 2.0, 4.0, 5.0)]);
    }

    #[test]
    fn structural_damage_tracks_paint_order_changes() {
        let a = DisplayCommand::FillRect {
            rect: Rect::new(0.0, 0.0, 4.0, 4.0),
            color: Color::BLACK,
        };
        let b = DisplayCommand::FillRect {
            rect: Rect::new(2.0, 0.0, 4.0, 4.0),
            color: Color::rgb(255, 0, 0),
        };
        let before = DisplayList {
            command_ids: vec![DisplayItemId::test(1), DisplayItemId::test(2)],
            commands: vec![a, b],
        };
        let after = DisplayList {
            command_ids: vec![DisplayItemId::test(2), DisplayItemId::test(1)],
            commands: vec![b, a],
        };
        let damage = DamageRegion::between(Some(&before), &after);

        assert!(damage.rects.contains(&Rect::new(0.0, 0.0, 4.0, 4.0)));
        assert!(damage.rects.contains(&Rect::new(2.0, 0.0, 4.0, 4.0)));
    }

'''
if test_marker not in s:
    raise SystemExit("structural damage test marker missing")
s = s.replace(test_marker, tests + test_marker, 1)

path.write_text(s)
