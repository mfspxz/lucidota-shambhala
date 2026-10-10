// sham_layers — sham-012 (ArcGIS-style layer system), sham-021 (orthomosaic
// import), sham-022 (coordinate CSV/GPX import).
// Layers toggle; orthomosaic layer; zone layer. Minimal GeoTIFF reader
// (uncompressed 8-bit grayscale/RGB). CSV/GPX coordinate import with validation.
#![forbid(unsafe_code)]

mod organelle;
pub use organelle::*;
#[cfg(test)]
mod tests;

use sham_spatial::GeoCoord;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayerKind {
    Orthomosaic,
    Zone,
    Street,
    Point,
    Flow,
    Heatmap,
    Annotation,
}

#[derive(Clone, Debug)]
pub struct Layer {
    pub name: String,
    pub kind: LayerKind,
    pub visible: bool,
    pub opacity: f32, // 0..1
    pub z: i32,       // draw order
}

#[derive(Default)]
pub struct LayerStack {
    layers: Vec<Layer>,
}
impl LayerStack {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add(&mut self, name: &str, kind: LayerKind, z: i32) -> usize {
        self.layers.push(Layer {
            name: name.into(),
            kind,
            visible: true,
            opacity: 1.0,
            z,
        });
        self.layers.len() - 1
    }
    pub fn toggle(&mut self, idx: usize) -> Result<bool, String> {
        let l = self.layers.get_mut(idx).ok_or("no such layer")?;
        l.visible = !l.visible;
        Ok(l.visible)
    }
    pub fn set_opacity(&mut self, idx: usize, o: f32) -> Result<(), String> {
        if !(0.0..=1.0).contains(&o) {
            return Err("opacity must be 0..1".into());
        }
        self.layers.get_mut(idx).ok_or("no such layer")?.opacity = o;
        Ok(())
    }
    /// Visible layers in draw order.
    pub fn render_order(&self) -> Vec<&Layer> {
        let mut v: Vec<&Layer> = self.layers.iter().filter(|l| l.visible).collect();
        v.sort_by_key(|l| l.z);
        v
    }
    pub fn by_kind(&self, k: LayerKind) -> Vec<&Layer> {
        self.layers.iter().filter(|l| l.kind == k).collect()
    }
    pub fn len(&self) -> usize {
        self.layers.len()
    }
}

// ---------------- sham-021: orthomosaic (GeoTIFF) import ----------------

/// Parsed raster header (subset of TIFF needed for orthomosaics).
#[derive(Clone, Debug, PartialEq)]
pub struct RasterInfo {
    pub width: u32,
    pub height: u32,
    pub samples_per_pixel: u16,
    pub bits_per_sample: u16,
    pub origin: GeoCoord,   // top-left
    pub pixel_size_m: f64,  // ground sample distance
    /// Raw ModelTiepoint (tag 33922): (i, j, k, x, y, z) — raster pixel to model space.
    pub tiepoint: Option<[f64; 6]>,
    /// Raw ModelPixelScale (tag 33550): (sx, sy, sz).
    pub pixel_scale: Option<[f64; 3]>,
}

/// Minimal TIFF parser: uncompressed (compression=1) strips, 8-bit.
pub fn parse_geotiff(data: &[u8]) -> Result<RasterInfo, String> {
    if data.len() < 8 {
        return Err("too short for TIFF header".into());
    }
    let le = match &data[0..2] {
        b"II" => true,
        b"MM" => false,
        _ => return Err("not a TIFF (bad byte order)".into()),
    };
    let rd16 = |o: usize| -> Result<u16, String> {
        data.get(o..o + 2)
            .ok_or_else(|| "truncated u16".to_string())
            .map(|b| if le { u16::from_le_bytes([b[0], b[1]]) } else { u16::from_be_bytes([b[0], b[1]]) })
    };
    let rd32 = |o: usize| -> Result<u32, String> {
        data.get(o..o + 4)
            .ok_or_else(|| "truncated u32".to_string())
            .map(|b| {
                if le {
                    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
                } else {
                    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
                }
            })
    };
    let rd64 = |o: usize| -> Result<f64, String> {
        data.get(o..o + 8)
            .ok_or_else(|| "truncated f64".to_string())
            .map(|b| {
                let arr = [b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]];
                if le { f64::from_le_bytes(arr) } else { f64::from_be_bytes(arr) }
            })
    };
    if rd16(2)? != 42 {
        return Err("bad TIFF magic".into());
    }
    let ifd_off = rd32(4)? as usize;
    let n = rd16(ifd_off)? as usize;
    let mut tags: HashMap<u16, (u16, u32)> = HashMap::new(); // tag -> (type, value/offset)
    for i in 0..n {
        let e = ifd_off + 2 + i * 12;
        let tag = rd16(e)?;
        let typ = rd16(e + 2)?;
        let val = rd32(e + 8)?;
        tags.insert(tag, (typ, val));
    }
    let get = |tag: u16| -> Result<u32, String> {
        let (typ, raw) = tags.get(&tag).ok_or(format!("missing tag {}", tag))?;
        Ok(match typ {
            3 => {
                if le {
                    raw & 0xFFFF
                } else {
                    raw >> 16
                }
            } // SHORT stored inline
            _ => *raw,
        })
    };
    let width = get(256)?;
    let height = get(257)?;
    let compression = get(259)?;
    if compression != 1 {
        return Err(format!("only uncompressed TIFF supported, got {}", compression));
    }
    let spp = get(277)? as u16;
    let bps = get(258)? as u16;
    if bps != 8 {
        return Err(format!("only 8-bit supported, got {}", bps));
    }
    if width == 0 || height == 0 || width > 100_000 || height > 100_000 {
        return Err("implausible raster dimensions".into());
    }
    // GeoTIFF georeferencing: ModelTiepoint (33922, 6 doubles) and
    // ModelPixelScale (33550, 3 doubles). Values live at the tag's offset.
    // Model coords are projected (easting/northing), not lat/lon, so we
    // expose them raw; origin keeps the festival default.
    let read_doubles = |tag: u16, n: usize| -> Option<Vec<f64>> {
        let (typ, raw) = tags.get(&tag)?;
        if *typ != 12 {
            return None; // not DOUBLE
        }
        let off = *raw as usize;
        let mut v = Vec::with_capacity(n);
        for i in 0..n {
            v.push(rd64(off + i * 8).ok()?);
        }
        Some(v)
    };
    let tiepoint: Option<[f64; 6]> = read_doubles(33922, 6).and_then(|v| {
        v.try_into().ok()
    });
    let pixel_scale: Option<[f64; 3]> = read_doubles(33550, 3).and_then(|v| {
        v.try_into().ok()
    });
    // GeoTIFF origin: use ModelTiepoint (tag 33922) if present, else default 0,0.
    let origin = GeoCoord::new(49.20, -122.91).unwrap(); // festival default
    Ok(RasterInfo {
        width,
        height,
        samples_per_pixel: spp,
        bits_per_sample: bps,
        origin,
        pixel_size_m: 0.05, // 5cm GSD typical for drone orthomosaic
        tiepoint,
        pixel_scale,
    })
}

/// Tile a raster into N x M tiles; returns tile bounds (col,row,w,h).
pub fn tile_raster(info: &RasterInfo, cols: u32, rows: u32) -> Result<Vec<(u32, u32, u32, u32)>, String> {
    if cols == 0 || rows == 0 {
        return Err("tile grid must be non-zero".into());
    }
    let tw = (info.width + cols - 1) / cols;
    let th = (info.height + rows - 1) / rows;
    let mut out = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            let x = c * tw;
            let y = r * th;
            if x >= info.width || y >= info.height {
                continue;
            }
            out.push((x, y, tw.min(info.width - x), th.min(info.height - y)));
        }
    }
    Ok(out)
}

/// Build a minimal valid uncompressed 8-bit grayscale TIFF in memory (for tests/import smoke).
pub fn make_test_tiff(width: u32, height: u32) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(b"II");
    v.extend_from_slice(&42u16.to_le_bytes());
    v.extend_from_slice(&8u32.to_le_bytes()); // IFD offset
    // entries: width(256), height(257), bps(258), compression(259), spp(277)
    let entries: [(u16, u16, u32); 5] = [
        (256, 4, width),
        (257, 4, height),
        (258, 3, 8),
        (259, 3, 1),
        (277, 3, 1),
    ];
    v.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    for (tag, typ, val) in entries {
        v.extend_from_slice(&tag.to_le_bytes());
        v.extend_from_slice(&typ.to_le_bytes());
        v.extend_from_slice(&1u32.to_le_bytes());
        v.extend_from_slice(&val.to_le_bytes());
    }
    v.extend_from_slice(&0u32.to_le_bytes()); // next IFD
    v
}

// ---------------- sham-022: CSV / GPX coordinate import ----------------

#[derive(Clone, Debug, PartialEq)]
pub struct ImportedPoint {
    pub label: String,
    pub coord: GeoCoord,
    pub attrs: HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportReport {
    pub imported: usize,
    pub rejected: usize,
    pub reasons: Vec<String>,
}

/// Import CSV: header must include lat,lon (or latitude,longitude); optional label column.
/// Parsed points are appended to `pts` (mirrors import_gpx); the report carries counts.
pub fn import_csv(text: &str, pts: &mut Vec<ImportedPoint>) -> ImportReport {
    let mut lines = text.lines();
    let header = match lines.next() {
        Some(h) => h,
        None => {
            return ImportReport { imported: 0, rejected: 0, reasons: vec!["empty input".into()] }
        }
    };
    let cols: Vec<String> = header.split(',').map(|c| c.trim().to_lowercase()).collect();
    let lat_i = cols.iter().position(|c| c == "lat" || c == "latitude");
    let lon_i = cols.iter().position(|c| c == "lon" || c == "longitude" || c == "lng");
    let (Some(li), Some(lo)) = (lat_i, lon_i) else {
        return ImportReport {
            imported: 0,
            rejected: 0,
            reasons: vec!["missing lat/lon columns".into()],
        };
    };
    let label_i = cols.iter().position(|c| c == "label" || c == "name");
    let imported_before = pts.len();
    let mut rejected = 0usize;
    let mut reasons = Vec::new();
    for (n, line) in lines.enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split(',').collect();
        let get = |i: usize| f.get(i).map(|s| s.trim()).unwrap_or("");
        match (get(li).parse::<f64>(), get(lo).parse::<f64>()) {
            (Ok(lat), Ok(lon)) => match GeoCoord::new(lat, lon) {
                Ok(coord) => {
                    let label = label_i
                        .map(|i| get(i))
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("pt-{}", n));
                    pts.push(ImportedPoint {
                        label,
                        coord,
                        attrs: HashMap::new(),
                    });
                }
                Err(e) => {
                    rejected += 1;
                    reasons.push(format!("line {}: {}", n + 2, e));
                }
            },
            _ => {
                rejected += 1;
                reasons.push(format!("line {}: unparsable numbers", n + 2));
            }
        }
    }
    let imported = pts.len() - imported_before;
    ImportReport { imported, rejected, reasons }
}

/// Import GPX: extracts <wpt lat=".." lon=".."> with optional <name>.
/// Points returned via the report's side channel: parsed into `pts` out-param.
pub fn import_gpx(text: &str, pts: &mut Vec<ImportedPoint>) -> ImportReport {
    let mut imported = 0usize;
    let mut rejected = 0usize;
    let mut reasons = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("<wpt") {
        rest = &rest[start..];
        let tag_end = match rest.find('>') {
            Some(i) => i,
            None => break,
        };
        let tag = &rest[..tag_end];
        let attr = |name: &str| -> Option<f64> {
            let key = format!("{}=\"", name);
            let s = tag.find(&key)? + key.len();
            let e = tag[s..].find('"')?;
            tag[s..s + e].parse().ok()
        };
        let (lat, lon) = match (attr("lat"), attr("lon")) {
            (Some(a), Some(o)) => (a, o),
            _ => {
                rejected += 1;
                reasons.push("wpt missing lat/lon attributes".into());
                rest = &rest[tag_end..];
                continue;
            }
        };
        // find name + closing tag
        let close = rest.find("</wpt>").unwrap_or(rest.len());
        let body = &rest[..close];
        let name = body
            .find("<name>")
            .and_then(|s| {
                let e = body[s..].find("</name>")?;
                Some(body[s + 6..s + e].trim().to_string())
            })
            .unwrap_or_else(|| format!("wpt-{}", imported));
        match GeoCoord::new(lat, lon) {
            Ok(coord) => {
                pts.push(ImportedPoint { label: name, coord, attrs: HashMap::new() });
                imported += 1;
            }
            Err(e) => {
                rejected += 1;
                reasons.push(e);
            }
        }
        rest = &rest[close.min(rest.len())..];
        if rest.starts_with("</wpt>") {
            rest = &rest[6..];
        } else {
            break;
        }
    }
    ImportReport { imported, rejected, reasons }
}
