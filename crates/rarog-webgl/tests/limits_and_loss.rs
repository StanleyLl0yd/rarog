use rarog_canvas::{CanvasLimits, CanvasRegistry};
use rarog_webgl::{WebGlContextLossReason, WebGlError, WebGlLimits, WebGlRegistry};

fn canvas() -> CanvasRegistry {
    CanvasRegistry::try_new(CanvasLimits {
        max_surfaces: 8,
        max_contexts: 8,
        max_pixels_per_surface: 64,
        max_total_pixels: 256,
        max_state_stack_depth: 4,
    })
    .unwrap()
}

fn limits() -> WebGlLimits {
    WebGlLimits {
        max_contexts: 2,
        max_resources_per_context: 3,
        max_resources: 3,
        max_buffer_bytes: 8,
        max_total_buffer_bytes: 12,
        max_texture_dimension: 4,
        max_texture_pixels: 8,
        max_total_texture_pixels: 12,
    }
}

#[test]
fn context_limit_recovers_without_identity_reuse() {
    let mut canvas = canvas();
    let a = canvas.create_surface(1, 1).unwrap();
    let b = canvas.create_surface(1, 1).unwrap();
    let c = canvas.create_surface(1, 1).unwrap();
    let mut webgl = WebGlRegistry::try_new(limits()).unwrap();

    let first = webgl.create_context(&mut canvas, a).unwrap();
    let second = webgl.create_context(&mut canvas, b).unwrap();
    assert!(matches!(
        webgl.create_context(&mut canvas, c),
        Err(WebGlError::ContextLimitExceeded { .. })
    ));

    webgl.destroy_context(&mut canvas, first).unwrap();
    let replacement = webgl.create_context(&mut canvas, c).unwrap();
    assert_ne!(first, replacement);
    assert!(replacement.serial() > second.serial());
}

#[test]
fn global_resource_limit_recovers_after_exact_destroy() {
    let mut canvas = canvas();
    let a = canvas.create_surface(1, 1).unwrap();
    let b = canvas.create_surface(1, 1).unwrap();
    let mut webgl = WebGlRegistry::try_new(limits()).unwrap();
    let ca = webgl.create_context(&mut canvas, a).unwrap();
    let cb = webgl.create_context(&mut canvas, b).unwrap();

    let a_buffer = webgl.create_buffer(ca, 1).unwrap();
    let a_texture = webgl.create_texture(ca, 1, 1).unwrap();
    let b_buffer = webgl.create_buffer(cb, 1).unwrap();
    assert!(matches!(
        webgl.create_texture(cb, 1, 1),
        Err(WebGlError::ResourceLimitExceeded { .. })
    ));

    webgl.destroy_buffer(ca, a_buffer).unwrap();
    let replacement = webgl.create_texture(cb, 1, 1).unwrap();
    assert!(webgl.texture(replacement).is_some());
    assert!(webgl.texture(a_texture).is_some());
    assert!(webgl.buffer(b_buffer).is_some());
}

#[test]
fn buffer_byte_limits_are_atomic_and_capacity_recovers() {
    let mut canvas = canvas();
    let a = canvas.create_surface(1, 1).unwrap();
    let b = canvas.create_surface(1, 1).unwrap();
    let mut webgl = WebGlRegistry::try_new(limits()).unwrap();
    let ca = webgl.create_context(&mut canvas, a).unwrap();
    let cb = webgl.create_context(&mut canvas, b).unwrap();

    assert!(matches!(
        webgl.create_buffer(ca, 9),
        Err(WebGlError::BufferByteLimitExceeded { .. })
    ));
    assert_eq!(webgl.total_buffer_bytes(), 0);

    let eight = webgl.create_buffer(ca, 8).unwrap();
    assert!(matches!(
        webgl.create_buffer(cb, 5),
        Err(WebGlError::TotalBufferByteLimitExceeded { .. })
    ));
    assert_eq!(webgl.total_buffer_bytes(), 8);

    let four = webgl.create_buffer(cb, 4).unwrap();
    assert_eq!(webgl.total_buffer_bytes(), 12);
    webgl.destroy_buffer(ca, eight).unwrap();
    assert_eq!(webgl.total_buffer_bytes(), 4);
    let replacement = webgl.create_buffer(ca, 8).unwrap();
    assert_ne!(eight, replacement);
    assert!(webgl.buffer(four).is_some());
}

#[test]
fn texture_dimension_pixel_and_aggregate_limits_recover() {
    let mut canvas = canvas();
    let a = canvas.create_surface(1, 1).unwrap();
    let b = canvas.create_surface(1, 1).unwrap();
    let mut webgl = WebGlRegistry::try_new(limits()).unwrap();
    let ca = webgl.create_context(&mut canvas, a).unwrap();
    let cb = webgl.create_context(&mut canvas, b).unwrap();

    assert!(matches!(
        webgl.create_texture(ca, 5, 1),
        Err(WebGlError::TextureDimensionLimitExceeded { .. })
    ));
    assert!(matches!(
        webgl.create_texture(ca, 3, 3),
        Err(WebGlError::TexturePixelLimitExceeded { .. })
    ));
    assert_eq!(webgl.total_texture_pixels(), 0);

    let eight = webgl.create_texture(ca, 4, 2).unwrap();
    let four = webgl.create_texture(cb, 2, 2).unwrap();
    assert_eq!(webgl.total_texture_pixels(), 12);
    assert!(matches!(
        webgl.create_texture(cb, 1, 1),
        Err(WebGlError::TotalTexturePixelLimitExceeded { .. })
    ));
    assert_eq!(webgl.total_texture_pixels(), 12);

    webgl.destroy_texture(cb, four).unwrap();
    let replacement = webgl.create_texture(cb, 2, 2).unwrap();
    assert_ne!(four, replacement);
    assert!(webgl.texture(eight).is_some());
}

#[test]
fn loss_and_destroy_retire_stale_resources_and_release_canvas_lease() {
    let mut canvas = canvas();
    let surface = canvas.create_surface(2, 2).unwrap();
    let mut webgl = WebGlRegistry::try_new(limits()).unwrap();
    let context = webgl.create_context(&mut canvas, surface).unwrap();
    let buffer = webgl.create_buffer(context, 8).unwrap();
    let texture = webgl.create_texture(context, 2, 2).unwrap();

    assert!(webgl
        .lose_context(context, WebGlContextLossReason::ResourcePressure)
        .unwrap());
    assert!(webgl.buffer(buffer).is_none());
    assert!(webgl.texture(texture).is_none());
    assert_eq!(webgl.total_buffer_bytes(), 0);
    assert_eq!(webgl.total_texture_pixels(), 0);
    assert!(matches!(
        webgl.create_texture(context, 1, 1),
        Err(WebGlError::ContextLost(id)) if id == context
    ));
    assert!(canvas.create_2d_context(surface).is_err());

    webgl.destroy_context(&mut canvas, context).unwrap();
    assert!(webgl.context(context).is_none());
    assert!(matches!(
        webgl.create_buffer(context, 1),
        Err(WebGlError::UnknownContext(id)) if id == context
    ));
    assert!(canvas.create_2d_context(surface).is_ok());
}

#[test]
fn foreign_registry_context_identity_never_grants_resource_authority() {
    let mut first_canvas = canvas();
    let first_surface = first_canvas.create_surface(1, 1).unwrap();
    let mut second_canvas = canvas();
    let second_surface = second_canvas.create_surface(1, 1).unwrap();
    let mut first = WebGlRegistry::try_new(limits()).unwrap();
    let mut second = WebGlRegistry::try_new(limits()).unwrap();
    let first_context = first
        .create_context(&mut first_canvas, first_surface)
        .unwrap();
    let second_context = second
        .create_context(&mut second_canvas, second_surface)
        .unwrap();

    assert_ne!(first_context.scope(), second_context.scope());
    assert!(matches!(
        first.create_buffer(second_context, 1),
        Err(WebGlError::UnknownContext(id)) if id == second_context
    ));
}
