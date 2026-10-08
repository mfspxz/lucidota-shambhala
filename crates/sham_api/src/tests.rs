use super::*;

fn api() -> Api {
    let mut a = Api::new();
    a.auth.issue("tok-lead", "lead");
    a
}
fn authed(method: Method, path: &str) -> Request {
    Request::new(method, path).bearer("tok-lead")
}

#[test]
fn health_no_auth() {
    let mut a = api();
    let r = a.handle(&Request::new(Method::Get, "/health"));
    assert_eq!(r.status, 200);
}
#[test]
fn zones_requires_auth() {
    let mut a = api();
    let r = a.handle(&Request::new(Method::Get, "/zones"));
    assert_eq!(r.status, 401);
}
#[test]
fn bad_token_rejected() {
    let mut a = api();
    let r = a.handle(&Request::new(Method::Get, "/zones").bearer("nope"));
    assert_eq!(r.status, 401);
}
#[test]
fn create_zone_roundtrip() {
    let mut a = api();
    let r = a.handle(&authed(Method::Post, "/zones").body("name=north&capacity=50"));
    assert_eq!(r.status, 201);
    let r2 = a.handle(&authed(Method::Get, "/zones"));
    assert!(r2.body.contains("north"));
    assert!(r2.body.contains("\"capacity\":50"));
}
#[test]
fn create_zone_bad_capacity() {
    let mut a = api();
    let r = a.handle(&authed(Method::Post, "/zones").body("name=x&capacity=0"));
    assert_eq!(r.status, 400);
}
#[test]
fn car_park_release_flow() {
    let mut a = api();
    a.handle(&authed(Method::Post, "/zones").body("name=z&capacity=5"));
    let zid = 1001u64; // first nid
    a.handle(&authed(Method::Post, "/cars").body("plate=BC123"));
    let cid = 1002u64;
    let rp = a.handle(&authed(Method::Post, "/park").body(&format!("car={}&zone={}", cid, zid)));
    assert_eq!(rp.status, 200);
    assert!(rp.body.contains("park"));
    let ro = a.handle(&authed(Method::Get, "/occupancy"));
    assert!(ro.body.contains("20.00"));
    let rr = a.handle(&authed(Method::Post, "/release").body(&format!("car={}", cid)));
    assert_eq!(rr.status, 200);
}
#[test]
fn park_unknown_car_400() {
    let mut a = api();
    let r = a.handle(&authed(Method::Post, "/park").body("car=1&zone=2"));
    assert_eq!(r.status, 400);
}
#[test]
fn receipts_endpoint() {
    let mut a = api();
    a.handle(&authed(Method::Post, "/zones").body("name=z&capacity=5"));
    a.handle(&authed(Method::Post, "/cars").body("plate=P1"));
    a.handle(&authed(Method::Post, "/park").body("car=1002&zone=1001"));
    let r = a.handle(&authed(Method::Get, "/receipts"));
    assert!(r.body.contains("\"op\":\"park\""));
}
#[test]
fn openapi_valid_shape() {
    let mut a = api();
    let r = a.handle(&authed(Method::Get, "/openapi.json"));
    assert_eq!(r.status, 200);
    assert!(r.body.contains("\"openapi\":\"3.0.0\""));
    assert!(r.body.contains("/park"));
    assert!(r.body.contains("bearerAuth"));
}
#[test]
fn not_found_404() {
    let mut a = api();
    let r = a.handle(&authed(Method::Get, "/nope"));
    assert_eq!(r.status, 404);
}
#[test]
fn api_port_is_8080() {
    assert_eq!(API_PORT, 8080);
}
#[test]
fn car_empty_plate_400() {
    let mut a = api();
    let r = a.handle(&authed(Method::Post, "/cars").body("plate="));
    assert_eq!(r.status, 400);
}
#[test]
fn auth_role_returned() {
    let a = api();
    let req = Request::new(Method::Get, "/zones").bearer("tok-lead");
    assert_eq!(a.auth.check(&req).unwrap(), "lead");
}
#[test]
fn parse_form_edge() {
    let m = parse_form("a=1&b=&c");
    assert_eq!(m["a"], "1");
    assert_eq!(m["b"], "");
    assert_eq!(m["c"], "");
}
#[test]
fn response_helpers() {
    assert_eq!(Response::ok("x").status, 200);
    assert_eq!(Response::created("x").status, 201);
    assert_eq!(Response::bad("x").status, 400);
    assert_eq!(Response::unauthorized().status, 401);
    assert_eq!(Response::not_found().status, 404);
}
#[test]
fn method_variants() {
    assert_ne!(Method::Get, Method::Post);
    assert_ne!(Method::Put, Method::Delete);
}
