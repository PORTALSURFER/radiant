use unicode_segmentation::UnicodeSegmentation;

pub(super) fn previous_grapheme_boundary(value: &str, scalar: usize) -> usize {
    boundaries(value)
        .into_iter()
        .rev()
        .find(|boundary| *boundary < scalar)
        .unwrap_or(0)
}

pub(super) fn next_grapheme_boundary(value: &str, scalar: usize) -> usize {
    let scalar = scalar.min(value.chars().count());
    boundaries(value)
        .into_iter()
        .find(|boundary| *boundary > scalar)
        .unwrap_or(scalar)
}

pub(super) fn boundary_at_or_before(value: &str, scalar: usize) -> usize {
    boundaries(value)
        .into_iter()
        .rev()
        .find(|boundary| *boundary <= scalar)
        .unwrap_or(0)
}

pub(super) fn boundary_at_or_after(value: &str, scalar: usize) -> usize {
    let scalar = scalar.min(value.chars().count());
    boundaries(value)
        .into_iter()
        .find(|boundary| *boundary >= scalar)
        .unwrap_or(scalar)
}

fn boundaries(value: &str) -> Vec<usize> {
    let mut result = Vec::with_capacity(value.chars().count().saturating_add(1));
    result.push(0);
    let mut scalar = 0;
    for grapheme in value.graphemes(true) {
        scalar += grapheme.chars().count();
        result.push(scalar);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{
        boundary_at_or_after, boundary_at_or_before, next_grapheme_boundary,
        previous_grapheme_boundary,
    };

    #[test]
    fn combining_and_non_bmp_clusters_have_indivisible_scalar_boundaries() {
        let text = "a e\u{301} \u{1f642}b";

        assert_eq!(previous_grapheme_boundary(text, 3), 2);
        assert_eq!(next_grapheme_boundary(text, 2), 4);
        assert_eq!(boundary_at_or_before(text, 3), 2);
        assert_eq!(boundary_at_or_after(text, 2), 2);
        assert_eq!(
            next_grapheme_boundary(text, text.chars().count()),
            text.chars().count()
        );
    }
}
