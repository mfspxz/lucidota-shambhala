use super::*;

// ---- sham-012: layer system ----
#[test]
fn layer_add_and_count() {
    let mut s = LayerStack::new();
    s.add("ortho", LayerKind::Orthomosaic, 0);
    s.add("zones", LayerKind::Zone, 10);
    assert_eq!(s.len(), 2);
}
#[test]
fn layer_toggle() {
    let mut s = LayerStack::new();
    let i = s.add("zones", LayerKind::Zone, 10);
    assert_eq!(s.toggle(i).unwrap(), false);
    assert_eq!(s.toggle(i).unwrap(), true);
}
#[test]
fn layer_toggle_bad_idx() {
    let mut s = LayerStack::new();
    assert!(s.toggle(9).is_err());
}
#[test]
fn render_order_by_z() {
    let mut s = LayerStack::new();
    s.add("zones", LayerKind::Zone, 10);
    s.add("ortho", LayerKind::Orthomosaic, 0);
    s.add("notes", LayerKind::Annotation, 20);
    let names: Vec<&str> = s.render_order().iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, vec!["ortho", "zones", "notes"]);
}
#[test]
fn render_order_skips_hidden() {
    let mut s = LayerStack::new();
    let i = s.add("zones", LayerKind::Zone, 10);
    s.add("ortho", LayerKind::Orthomosaic, 0);
    s.toggle(i).unwrap();
    assert_eq!(s.render_order().len(), 1);
}
#[test]
fn opacity_set_and_validate() {
    let mut s = LayerStack::new();
    let i = s.add("ortho", LayerKind::Orthomosaic, 0);
    s.set_opacity(i, 0.5).unwrap();
    assert!(s.set_opacity(i, 1.5).is_err());
    assert!(s.set_opacity(i, -0.1).is_err());
    assert!(s.set_opacity(99, 0.5).is_err());
}
#[test]
fn by_kind_filters() {
    let mut s = LayerStack::new();
    s.add("ortho1", LayerKind::Orthomosaic, 0);
    s.add("ortho2", LayerKind::Orthomosaic, 1);
    s.add("zones", LayerKind::Zone, 10);
    assert_eq!(s.by_kind(LayerKind::Orthomosaic).len(), 2);
    assert_eq!(s.by_kind(LayerKind::Flow).len(), 0);
}
#[test]
fn layer_defaults_visible() {
    let mut s = LayerStack::new();
    s.add("x", LayerKind::Point, 5);
    assert!(s.render_order()[0].visible);
}

// ---- sham-021: orthomosaic import ----
#[test]
fn tiff_roundtrip() {
    let data = make_test_tiff(2048, 1024);
    let info = parse_geotiff(&data).unwrap();
    assert_eq!((info.width, info.height), (2048, 1024));
    assert_eq!(info.samples_per_pixel, 1);
    assert_eq!(info.bits_per_sample, 8);
}
#[test]
fn tiff_rejects_garbage() {
    assert!(parse_geotiff(b"not a tiff at all..........").is_err());
}
#[test]
fn tiff_rejects_short() {
    assert!(parse_geotiff(b"II").is_err());
}
#[test]
fn tiff_rejects_bad_magic() {
    let mut d = make_test_tiff(10, 10);
    d[2] = 0x2B;
    d[3] = 0x00;
    assert!(parse_geotiff(&d).is_err());
}
#[test]
fn tiff_rejects_compressed() {
    let mut d = make_test_tiff(10, 10);
    // compression entry is the 4th (index 3): offset 8+2+3*12+8 = 54
    let off = 8 + 2 + 3 * 12 + 8;
    d[off] = 5; // compression=5 (LZW)
    assert!(parse_geotiff(&d).unwrap_err().contains("uncompressed"));
}
#[test]
fn tiff_rejects_zero_dims() {
    let d = make_test_tiff(0, 10);
    assert!(parse_geotiff(&d).is_err());
}
#[test]
fn tiff_big_endian() {
    // hand-built minimal BE tiff
    let mut v = Vec::new();
    v.extend_from_slice(b"MM");
    v.extend_from_slice(&42u16.to_be_bytes());
    v.extend_from_slice(&8u32.to_be_bytes());
    let entries: [(u16, u16, u32); 5] = [
        (256, 4, 100),
        (257, 4, 50),
        (258, 3, 8 << 16), // BE SHORT inline: value in high bytes
        (259, 3, 1 << 16),
        (277, 3, 1 << 16),
    ];
    v.extend_from_slice(&(entries.len() as u16).to_be_bytes());
    for (tag, typ, val) in entries {
        v.extend_from_slice(&tag.to_be_bytes());
        v.extend_from_slice(&typ.to_be_bytes());
        v.extend_from_slice(&1u32.to_be_bytes());
        v.extend_from_slice(&val.to_be_bytes());
    }
    v.extend_from_slice(&0u32.to_be_bytes());
    let info = parse_geotiff(&v).unwrap();
    assert_eq!((info.width, info.height), (100, 50));
    assert_eq!(info.bits_per_sample, 8);
}
#[test]
fn tile_raster_even() {
    let info = parse_geotiff(&make_test_tiff(1000, 500)).unwrap();
    let tiles = tile_raster(&info, 4, 2).unwrap();
    assert_eq!(tiles.len(), 8);
    assert_eq!(tiles[0], (0, 0, 250, 250));
}
#[test]
fn tile_raster_uneven_covers_all() {
    let info = parse_geotiff(&make_test_tiff(1000, 500)).unwrap();
    let tiles = tile_raster(&info, 3, 3).unwrap();
    let area: u32 = tiles.iter().map(|(_, _, w, h)| w * h).sum();
    assert_eq!(area, 1000 * 500);
}
#[test]
fn tile_raster_bad_grid() {
    let info = parse_geotiff(&make_test_tiff(100, 100)).unwrap();
    assert!(tile_raster(&info, 0, 2).is_err());
}
#[test]
fn raster_gsd_sane() {
    let info = parse_geotiff(&make_test_tiff(100, 100)).unwrap();
    assert!(info.pixel_size_m > 0.0 && info.pixel_size_m < 1.0);
}

// ---- sham-022: CSV/GPX import ----
#[test]
fn csv_basic() {
    let mut pts = Vec::new();
    let r = import_csv("label,lat,lon\ngate-a,49.2,-122.9\nlot-1,49.21,-122.91\n", &mut pts);
    assert_eq!((r.imported, r.rejected), (2, 0));
    assert_eq!(pts.len(), 2);
    assert_eq!(pts[0].label, "gate-a");
    assert!((pts[0].coord.lat - 49.2).abs() < 1e-9);
    assert!((pts[1].coord.lon - -122.91).abs() < 1e-9);
}
#[test]
fn csv_latitude_longitude_headers() {
    let mut pts = Vec::new();
    let r = import_csv("latitude,longitude\n49.2,-122.9\n", &mut pts);
    assert_eq!(r.imported, 1);
    assert_eq!(pts.len(), 1);
}
#[test]
fn csv_missing_cols() {
    let mut pts = Vec::new();
    let r = import_csv("name,x\nfoo,1\n", &mut pts);
    assert_eq!(r.imported, 0);
    assert!(pts.is_empty());
    assert!(r.reasons.iter().any(|s| s.contains("lat/lon")));
}
#[test]
fn csv_bad_rows_rejected() {
    let mut pts = Vec::new();
    let r = import_csv("lat,lon\n49.2,-122.9\nnope,xx\n95.0,0.0\n", &mut pts);
    assert_eq!((r.imported, r.rejected), (1, 2));
    assert_eq!(r.reasons.len(), 2);
    assert_eq!(pts.len(), 1);
}
#[test]
fn csv_empty() {
    let mut pts = Vec::new();
    let r = import_csv("", &mut pts);
    assert_eq!(r.imported, 0);
    assert!(pts.is_empty());
}
#[test]
fn csv_blank_lines_skipped() {
    let mut pts = Vec::new();
    let r = import_csv("lat,lon\n\n49.2,-122.9\n\n", &mut pts);
    assert_eq!((r.imported, r.rejected), (1, 0));
    assert_eq!(pts.len(), 1);
}
#[test]
fn csv_appends_to_existing() {
    let mut pts = vec![ImportedPoint {
        label: "pre".into(),
        coord: GeoCoord::new(0.0, 0.0).unwrap(),
        attrs: Default::default(),
    }];
    let r = import_csv("lat,lon\n49.2,-122.9\n", &mut pts);
    assert_eq!(r.imported, 1);
    assert_eq!(pts.len(), 2); // appends, does not clear
    assert_eq!(pts[0].label, "pre");
}
#[test]
fn gpx_basic() {
    let mut pts = Vec::new();
    let r = import_gpx(
        "<gpx><wpt lat=\"49.2\" lon=\"-122.9\"><name>gate</name></wpt><wpt lat=\"49.21\" lon=\"-122.91\"/></gpx>",
        &mut pts,
    );
    assert_eq!((r.imported, r.rejected), (2, 0));
    assert_eq!(pts[0].label, "gate");
    assert_eq!(pts[1].label, "wpt-1");
}
#[test]
fn gpx_bad_coords_rejected() {
    let mut pts = Vec::new();
    let r = import_gpx("<gpx><wpt lat=\"999\" lon=\"0\"/></gpx>", &mut pts);
    assert_eq!((r.imported, r.rejected), (0, 1));
}
#[test]
fn gpx_missing_attrs() {
    let mut pts = Vec::new();
    let r = import_gpx("<gpx><wpt lat=\"49.2\"/></gpx>", &mut pts);
    assert_eq!(r.rejected, 1);
}
#[test]
fn imported_point_attrs() {
    let p = ImportedPoint {
        label: "x".into(),
        coord: GeoCoord::new(49.0, -123.0).unwrap(),
        attrs: [("src".to_string(), "csv".to_string())].into_iter().collect(),
    };
    assert_eq!(p.attrs["src"], "csv");
}
#[test]
fn geotiff_tiepoint_and_scale_parsed() {
    // build a TIFF with ModelTiepoint (33922) + ModelPixelScale (33550)
    let mut v = make_test_tiff(100, 80);
    // rewrite with extra tags: strip the trailing next-IFD u32, append tags
    v.truncate(v.len() - 4);
    let tie_off = v.len() as u32 + 4 + 2 + 2 * 12 + 4; // after new IFD
    let scale_off = tie_off + 48;
    let mut ifd = Vec::new();
    ifd.extend_from_slice(&7u16.to_le_bytes()); // 5 base + 2 geo tags
    // base tags (copied layout from make_test_tiff)
    for (tag, typ, val) in [(256u16, 4u16, 100u32), (257, 4, 80), (258, 3, 8), (259, 3, 1), (277, 3, 1)] {
        ifd.extend_from_slice(&tag.to_le_bytes());
        ifd.extend_from_slice(&typ.to_le_bytes());
        ifd.extend_from_slice(&1u32.to_le_bytes());
        ifd.extend_from_slice(&val.to_le_bytes());
    }
    // 33922 DOUBLE count 6 @ tie_off
    ifd.extend_from_slice(&33922u16.to_le_bytes());
    ifd.extend_from_slice(&12u16.to_le_bytes());
    ifd.extend_from_slice(&6u32.to_le_bytes());
    ifd.extend_from_slice(&tie_off.to_le_bytes());
    // 33550 DOUBLE count 3 @ scale_off
    ifd.extend_from_slice(&33550u16.to_le_bytes());
    ifd.extend_from_slice(&12u16.to_le_bytes());
    ifd.extend_from_slice(&3u32.to_le_bytes());
    ifd.extend_from_slice(&scale_off.to_le_bytes());
    ifd.extend_from_slice(&0u32.to_le_bytes());
    // patch entry count + append IFD at offset 8
    let mut full = Vec::new();
    full.extend_from_slice(b"II");
    full.extend_from_slice(&42u16.to_le_bytes());
    full.extend_from_slice(&8u32.to_le_bytes());
    full.extend_from_slice(&ifd);
    // pad to tie_off then write doubles
    while full.len() < tie_off as usize {
        full.push(0);
    }
    for d in [0.0f64, 0.0, 0.0, 500_000.0, 5_500_000.0, 0.0] {
        full.extend_from_slice(&d.to_le_bytes());
    }
    for d in [0.05f64, 0.05, 0.0] {
        full.extend_from_slice(&d.to_le_bytes());
    }
    let info = parse_geotiff(&full).unwrap();
    let tp = info.tiepoint.expect("tiepoint parsed");
    assert!((tp[3] - 500_000.0).abs() < 1e-6);
    assert!((tp[4] - 5_500_000.0).abs() < 1e-6);
    let ps = info.pixel_scale.expect("pixel scale parsed");
    assert!((ps[0] - 0.05).abs() < 1e-9);
}
#[test]
fn geotiff_without_geotags_still_parses() {
    let info = parse_geotiff(&make_test_tiff(64, 64)).unwrap();
    assert!(info.tiepoint.is_none());
    assert!(info.pixel_scale.is_none());
}
