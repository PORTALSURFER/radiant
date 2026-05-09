//! SVG numeric and transform parsing helpers.

use vello::kurbo::{Affine, Point as KurboPoint, Vec2};

pub(super) fn parse_attr_f64(node: roxmltree::Node<'_, '_>, attr: &str) -> Option<f64> {
    parse_number(node.attribute(attr)?)
}

pub(super) fn parse_transform_list(raw: Option<&str>) -> Option<Affine> {
    let Some(mut remaining) = raw.map(str::trim) else {
        return Some(Affine::IDENTITY);
    };
    if remaining.is_empty() {
        return Some(Affine::IDENTITY);
    }

    let mut transform = Affine::IDENTITY;
    while !remaining.is_empty() {
        remaining = remaining.trim_start_matches(|ch: char| ch.is_ascii_whitespace() || ch == ',');
        if remaining.is_empty() {
            break;
        }
        let open = remaining.find('(')?;
        let name = remaining[..open].trim();
        let body = &remaining[open + 1..];
        let close = body.find(')')?;
        let args = &body[..close];
        remaining = &body[close + 1..];
        transform *= parse_single_transform(name, args)?;
    }
    Some(transform)
}

fn parse_single_transform(name: &str, args: &str) -> Option<Affine> {
    let values = parse_number_list(args)?;
    match name {
        "matrix" if values.len() == 6 => Some(Affine::new([
            values[0], values[1], values[2], values[3], values[4], values[5],
        ])),
        "translate" if values.len() == 1 => Some(Affine::translate(Vec2::new(values[0], 0.0))),
        "translate" if values.len() == 2 => {
            Some(Affine::translate(Vec2::new(values[0], values[1])))
        }
        "scale" if values.len() == 1 => Some(Affine::scale(values[0])),
        "scale" if values.len() == 2 => Some(Affine::scale_non_uniform(values[0], values[1])),
        "rotate" if values.len() == 1 => Some(Affine::rotate(values[0].to_radians())),
        "rotate" if values.len() == 3 => {
            let center = KurboPoint::new(values[1], values[2]);
            Some(
                Affine::translate(center.to_vec2())
                    * Affine::rotate(values[0].to_radians())
                    * Affine::translate(-center.to_vec2()),
            )
        }
        "skewX" if values.len() == 1 => Some(Affine::new([
            1.0,
            0.0,
            values[0].to_radians().tan(),
            1.0,
            0.0,
            0.0,
        ])),
        "skewY" if values.len() == 1 => Some(Affine::new([
            1.0,
            values[0].to_radians().tan(),
            0.0,
            1.0,
            0.0,
            0.0,
        ])),
        _ => None,
    }
}

pub(super) fn parse_points(points: &str) -> Option<Vec<KurboPoint>> {
    let coords = parse_number_list(points)?;
    if coords.len() < 6 || coords.len() % 2 != 0 {
        return None;
    }
    Some(
        coords
            .chunks_exact(2)
            .map(|pair| KurboPoint::new(pair[0], pair[1]))
            .collect(),
    )
}

pub(super) fn parse_number_list(raw: &str) -> Option<Vec<f64>> {
    let normalized = raw.replace(',', " ");
    normalized
        .split_whitespace()
        .map(parse_number)
        .collect::<Option<Vec<_>>>()
}

fn parse_number(raw: &str) -> Option<f64> {
    raw.trim().parse::<f64>().ok()
}
