from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


resources_path = Path("crates/rarog-resources/src/lib.rs")
resources = resources_path.read_text()
resources = replace_once(
    resources,
    '''        let entry = self
            .entries
            .get(&id)
            .ok_or(ImageResourceError::UnknownResource(id))?;
        if !matches!(entry.state, StoredImageState::Pending) {
            return Err(ImageResourceError::InvalidState(id));
        }
        let revision = entry
            .revision
            .checked_add(1)
            .ok_or(ImageResourceError::RevisionExhausted(id))?;
        let pixels = image.pixel_count();
        let total = self.validate_pixels(pixels, 0)?;
        let entry = self
            .entries
            .get_mut(&id)
            .expect("validated image resource exists");
        entry.revision = revision;
        entry.state = StoredImageState::Ready(image);
        self.total_pixels = total;
''',
    '''        let limits = self.limits;
        let total_pixels = self.total_pixels;
        let entry = self
            .entries
            .get_mut(&id)
            .ok_or(ImageResourceError::UnknownResource(id))?;
        if !matches!(entry.state, StoredImageState::Pending) {
            return Err(ImageResourceError::InvalidState(id));
        }
        let revision = entry
            .revision
            .checked_add(1)
            .ok_or(ImageResourceError::RevisionExhausted(id))?;
        let total = validate_pixels(limits, total_pixels, image.pixel_count(), 0)?;
        entry.revision = revision;
        entry.state = StoredImageState::Ready(image);
        self.total_pixels = total;
''',
    "resolve one lookup",
)
resources = replace_once(
    resources,
    '''        let entry = self
            .entries
            .get(&id)
            .ok_or(ImageResourceError::UnknownResource(id))?;
        let old_pixels = match &entry.state {
            StoredImageState::Ready(image) => image.pixel_count(),
            StoredImageState::Pending | StoredImageState::Failed => {
                return Err(ImageResourceError::InvalidState(id));
            }
        };
        let revision = entry
            .revision
            .checked_add(1)
            .ok_or(ImageResourceError::RevisionExhausted(id))?;
        let total = self.validate_pixels(image.pixel_count(), old_pixels)?;
        let entry = self
            .entries
            .get_mut(&id)
            .expect("validated image resource exists");
        entry.revision = revision;
        entry.state = StoredImageState::Ready(image);
        self.total_pixels = total;
''',
    '''        let limits = self.limits;
        let total_pixels = self.total_pixels;
        let entry = self
            .entries
            .get_mut(&id)
            .ok_or(ImageResourceError::UnknownResource(id))?;
        let old_pixels = match &entry.state {
            StoredImageState::Ready(image) => image.pixel_count(),
            StoredImageState::Pending | StoredImageState::Failed => {
                return Err(ImageResourceError::InvalidState(id));
            }
        };
        let revision = entry
            .revision
            .checked_add(1)
            .ok_or(ImageResourceError::RevisionExhausted(id))?;
        let total = validate_pixels(limits, total_pixels, image.pixel_count(), old_pixels)?;
        entry.revision = revision;
        entry.state = StoredImageState::Ready(image);
        self.total_pixels = total;
''',
    "replace ready one lookup",
)
method_start = resources.index("    fn validate_pixels(\n")
impl_end = resources.index("\n}\n\n#[cfg(test)]", method_start)
resources = resources[:method_start] + resources[impl_end:]
helper = '''
fn validate_pixels(
    limits: ImageResourceLimits,
    total_pixels: u64,
    new_pixels: u64,
    replacing_pixels: u64,
) -> Result<u64, ImageResourceError> {
    if new_pixels > limits.max_pixels_per_resource {
        return Err(ImageResourceError::ImagePixelLimitExceeded {
            pixels: new_pixels,
            limit: limits.max_pixels_per_resource,
        });
    }
    let retained = total_pixels.saturating_sub(replacing_pixels);
    let total = retained
        .checked_add(new_pixels)
        .ok_or(ImageResourceError::PixelCountOverflow)?;
    if total > limits.max_total_pixels {
        return Err(ImageResourceError::TotalPixelLimitExceeded {
            pixels: total,
            limit: limits.max_total_pixels,
        });
    }
    Ok(total)
}
'''
insert_at = resources.index("\n#[cfg(test)]")
resources = resources[:insert_at] + "\n" + helper + resources[insert_at:]
resources_path.write_text(resources)

layout_path = Path("crates/rarog-layout/src/lib.rs")
layout = layout_path.read_text()
layout = replace_once(
    layout,
    '''        let face = if inherited {
            runs.last().map(|run| run.face)
        } else {
            chain.select_face_for_characters(&characters, range)
        }
        .or_else(|| chain.faces.last().map(|face| face.id))
        .expect("font fallback chain must contain at least one face");
''',
    '''        let face = if inherited {
            runs.last().map(|run| run.face)
        } else {
            chain.select_face_for_characters(&characters, range)
        }
        .or_else(|| chain.faces.last().map(|face| face.id));
        let Some(face) = face else {
            return Vec::new();
        };
''',
    "empty fallback chain",
)
layout += '''

#[cfg(test)]
mod audit_font_fallback_tests {
    use super::*;

    #[test]
    fn empty_font_fallback_chain_is_non_panicking() {
        let chain = FontFallbackChain { faces: Vec::new() };
        assert!(font_runs("Rarog", &chain).is_empty());
        assert!(shaping_runs("Rarog", &chain).is_empty());
    }
}
'''
layout_path.write_text(layout)

text_path = Path("crates/rarog-text-opentype/src/lib.rs")
text = text_path.read_text()
text = replace_once(
    text,
    '''    let mut starts = infos.iter().map(|info| info.cluster).collect::<Vec<_>>();
    if starts.iter().any(|cluster| *cluster >= text_len) && text_len != 0 {
        return Err(OpenTypeShapingError::InvalidClusterBoundary(
            starts
                .into_iter()
                .find(|cluster| *cluster >= text_len)
                .unwrap_or(text_len),
        ));
    }
''',
    '''    let mut starts = infos.iter().map(|info| info.cluster).collect::<Vec<_>>();
    if text_len != 0 {
        if let Some(cluster) = starts.iter().copied().find(|cluster| *cluster >= text_len) {
            return Err(OpenTypeShapingError::InvalidClusterBoundary(cluster));
        }
    }
''',
    "cluster validation scan",
)
text_path.write_text(text)
