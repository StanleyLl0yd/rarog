from pathlib import Path


def replace(path: str, old: str, new: str, count: int = 1) -> None:
    file = Path(path)
    text = file.read_text(encoding="utf-8")
    found = text.count(old)
    if found != count:
        raise SystemExit(f"{path}: expected {count}, found {found}: {old[:120]!r}")
    file.write_text(text.replace(old, new), encoding="utf-8")


paint = "crates/rarog-paint/src/lib.rs"
replace(
    paint,
    """#[derive(Clone, Debug, Default, PartialEq)]
pub struct DisplayList {
    pub command_ids: Vec<DisplayItemId>,
    pub commands: Vec<DisplayCommand>,
}

impl DisplayList {
    fn push(&mut self, id: DisplayItemId, command: DisplayCommand) {""",
    """#[derive(Clone, Debug, Default, PartialEq)]
pub struct DisplayList {
    command_ids: Vec<DisplayItemId>,
    commands: Vec<DisplayCommand>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayListError {
    LengthMismatch { ids: usize, commands: usize },
    DuplicateIds,
    UnbalancedStructure,
}

impl fmt::Display for DisplayListError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthMismatch { ids, commands } => {
                write!(formatter, "display list has {ids} IDs for {commands} commands")
            }
            Self::DuplicateIds => formatter.write_str("display list contains duplicate item IDs"),
            Self::UnbalancedStructure => {
                formatter.write_str("display list structural scopes are invalid")
            }
        }
    }
}

impl std::error::Error for DisplayListError {}

impl DisplayList {
    pub fn try_from_parts(
        command_ids: Vec<DisplayItemId>,
        commands: Vec<DisplayCommand>,
    ) -> Result<Self, DisplayListError> {
        let list = Self {
            command_ids,
            commands,
        };
        list.validate()?;
        Ok(list)
    }

    pub fn command_ids(&self) -> &[DisplayItemId] {
        &self.command_ids
    }

    pub fn commands(&self) -> &[DisplayCommand] {
        &self.commands
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn validate(&self) -> Result<(), DisplayListError> {
        if self.command_ids.len() != self.commands.len() {
            return Err(DisplayListError::LengthMismatch {
                ids: self.command_ids.len(),
                commands: self.commands.len(),
            });
        }
        if !self.has_unique_ids() {
            return Err(DisplayListError::DuplicateIds);
        }
        if !self.has_balanced_structure() {
            return Err(DisplayListError::UnbalancedStructure);
        }
        Ok(())
    }

    fn push(&mut self, id: DisplayItemId, command: DisplayCommand) {""",
)
replace(
    paint,
    """pub fn build_display_list_for_fragment(fragment: &Fragment) -> DisplayList {
    let mut list = DisplayList::default();
    collect(fragment, &mut list);
    assert!(list.has_unique_ids(), "display item IDs must be unique");
    assert!(
        list.has_balanced_structure(),
        "display list structural scopes must be balanced"
    );
    list
}""",
    """pub fn build_display_list_for_fragment(fragment: &Fragment) -> DisplayList {
    let mut list = DisplayList::default();
    collect(fragment, &mut list);
    list
}""",
)
replace(
    paint,
    """    pub fn between(previous: Option<&DisplayList>, current: &DisplayList) -> Self {
        assert!(
            current.has_unique_ids(),
            "current display list contains duplicate display item IDs"
        );
        if let Some(previous) = previous {
            assert!(
                previous.has_unique_ids(),
                "previous display list contains duplicate display item IDs"
            );
        }

        let Some(previous) = previous else {""",
    """    pub fn between(previous: Option<&DisplayList>, current: &DisplayList) -> Self {
        let Some(previous) = previous else {""",
)
replace(
    paint,
    """impl Framebuffer {
    pub fn new(size: Size, background: Color) -> Self {
        Self::try_new(size, background)
            .expect("framebuffer dimensions must fit the R0 safety budget")
    }

    pub fn try_new(size: Size, background: Color) -> Result<Self, FramebufferError> {""",
    """impl Framebuffer {
    #[cfg(test)]
    fn new(size: Size, background: Color) -> Self {
        Self::try_new(size, background).expect("test framebuffer dimensions are valid")
    }

    pub fn try_new(size: Size, background: Color) -> Result<Self, FramebufferError> {""",
)
replace(
    paint,
    """    pub fn rasterize(&mut self, list: &DisplayList) {
        assert!(
            list.has_balanced_structure(),
            "display list structural scopes must be balanced"
        );
        let framebuffer_clip = Rect::new(0.0, 0.0, self.width as f32, self.height as f32);""",
    """    pub fn rasterize(&mut self, list: &DisplayList) {
        let framebuffer_clip = Rect::new(0.0, 0.0, self.width as f32, self.height as f32);""",
)
# Replace the panic-based duplicate-ID damage test with constructor validation coverage.
old = """    #[test]
    #[should_panic(expected = \"current display list contains duplicate display item IDs\")]
    fn damage_rejects_duplicate_display_ids() {
        let id = DisplayItemId {
            source: 1,
            fragment: 2,
            slot: 0,
        };
        let list = DisplayList {
            command_ids: vec![id, id],
            commands: vec![
                DisplayCommand::FillRect {
                    rect: Rect::new(0.0, 0.0, 1.0, 1.0),
                    color: Color::BLACK,
                },
                DisplayCommand::FillRect {
                    rect: Rect::new(1.0, 0.0, 1.0, 1.0),
                    color: Color::BLACK,
                },
            ],
        };
        let _ = DamageRegion::between(None, &list);
    }
"""
new = """    #[test]
    fn malformed_external_display_lists_are_rejected_without_panicking() {
        let id = DisplayItemId {
            source: 1,
            fragment: 2,
            slot: 0,
        };
        let command = DisplayCommand::FillRect {
            rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            color: Color::BLACK,
        };

        assert_eq!(
            DisplayList::try_from_parts(vec![id], vec![command, command]),
            Err(DisplayListError::LengthMismatch {
                ids: 1,
                commands: 2
            })
        );
        assert_eq!(
            DisplayList::try_from_parts(vec![id, id], vec![command, command]),
            Err(DisplayListError::DuplicateIds)
        );
        assert_eq!(
            DisplayList::try_from_parts(
                vec![id],
                vec![DisplayCommand::PopClip],
            ),
            Err(DisplayListError::UnbalancedStructure)
        );
    }
"""
replace(paint, old, new)

engine = "crates/rarog-engine/src/lib.rs"
text = Path(engine).read_text(encoding="utf-8")
text = text.replace("self.display_list.commands.len()", "self.display_list.len()")
text = text.replace("display_list.commands.len()", "display_list.len()")
Path(engine).write_text(text, encoding="utf-8")

embedder = "crates/rarog-engine/src/embedder.rs"
text = Path(embedder).read_text(encoding="utf-8")
text = text.replace("frame.display_list.commands.is_empty()", "frame.display_list.is_empty()")
text = text.replace("frame.display_list.commands.len()", "frame.display_list.len()")
Path(embedder).write_text(text, encoding="utf-8")
