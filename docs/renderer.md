# The renderer, one frame at a time

This is a walkthrough of Lantern's renderer for someone who wants to build one like it. It follows a single frame
from the game to the screen, names the file each step lives in, and ends with what this renderer leaves out and
where each missing feature would go.

Read `engine-structure.md` first for where the folders sit, and `application-and-game-loop.md` for when `Render` is
called.

## The shape in one picture

```text
  game code (Nyx)                 engine (Nox)                                   GPU
  ---------------                 ------------                                   ---
  OnRender hook  ──fills──▶  RenderWorld          ──Render()──▶  RenderBackend
  entity world   ──ExtractDraws─▶  (plain data:                   │
  HUD, menus     ──overlay quads─▶   draws, lines,                ├─ GpuRenderer  ──SDL GPU──▶  Vulkan / Metal / D3D12
                                     overlay, camera,             │   (sdl-facade/renderer)
                                     lights, sky, fog,            └─ HeadlessRenderer (tests)
                                     post settings)
```

Two ideas carry the whole design:

1. **The frame is data.** Nothing above the renderer calls a GPU function. Each frame, the game fills a
   `RenderWorld`: a list of draw items, a list of debug lines, a list of 2D overlay vertices, and the camera, the sun and
   local lights, the sky, the fog, the shadow and post-processing settings and the clear colour. The renderer reads it and forgets it.
2. **One small interface between "what" and "how".** `RenderBackend` (`engine/src/renderer/RenderBackend.nox`) has
   seven methods: its driver name, create and remove a mesh, create and remove a texture, render a frame, and
   capture a frame to memory. `GpuRenderer` implements it over SDL GPU; `HeadlessRenderer` implements it with no
   GPU at all, for tests. A Vulkan or Direct3D renderer written by hand would be a third implementation, and nothing
   above the interface would change.

## The frame at a glance

One frame, in the order the GPU does it. The numbers match the steps under "4. Render" below.

```text
 CPU, before any GPU work
   cull draws against the camera's frustum, sort them, pick the 8 nearest local lights,
   plan the shadow tiles (ShadowPlan: plain data)

 GPU, one command buffer
                                                          writes                     skipped when
   1  sun shadow pass       3 cascades, one depth map     sun depth map              shadows off
   2  local shadow pass     up to 12 tiles, one map       local depth map            no shadowed light
   3  scene pass            lit meshes, then the sky      HDR scene (RGBA16F), depth never
   4  bloom                 5 shrinks, 4 growths          bloom chain                bloom off
   5  ambient occlusion     estimate, then blur           occlusion (R8)             off, or orthographic camera
   6  composite             x occlusion, blob shadows,    screen image               never
                            height fog, light shafts,     (or an LDR texture
                            + bloom, x exposure,           when FXAA is on)
                            tone map, colour grade
   7  FXAA                  smooths the composite         screen image               anti-aliasing off
   8  overlay pass          debug lines and particles,    screen image               nothing to draw
                            then the HUD, menus, text
```

Three rules explain the order:

- **Light stays unclamped until the composite.** The scene pass writes linear light into a 16-bit float texture, so a
  lamp or a gem can be far brighter than 1. Everything that adds or removes light (occlusion, fog, shafts, bloom,
  exposure) runs before the tone map, which squeezes the result into the 0 to 1 a display shows. See
  `bloom-and-tone-mapping.md`.
- **Effects that need depth read the scene pass's depth buffer.** Ambient occlusion, fog, light shafts and blob shadows
  rebuild each pixel's position from it, so the scene pass keeps its depth.
- **What must reach the screen exactly as written comes last.** Debug lines, particles and the overlay are drawn after
  the composite and FXAA, so they are neither tone mapped, graded, fogged nor smoothed.

The effects and the quality levels that switch them are in `graphics-effects.md`, with the cost of each in
"Quality levels and cost" below.

## The files

| File | What it holds |
| --- | --- |
| `renderer/RenderBackend.nox` | The `RenderBackend` interface and `RenderReport` |
| `renderer/RenderWorld.nox` | `RenderWorld` (one frame), `DrawItem`, `Material`, `DirectionalLight` (the sun), `Light` (point and spot), `LineVertex`, the 2D `Overlay` |
| `renderer/ShadowPlan.nox` | `ShadowPlan`: which shadow tiles to draw and from where (the sun's cascades, each shadowed light's tiles), as plain data |
| `renderer/Particles.nox` | `ParticleSystem`: dust, sparks and trails, simulated on the CPU and drawn as line streaks |
| `renderer/Camera.nox` | `Camera`: position, forward, field of view, near/far planes, view and projection matrices |
| `renderer/Frustum.nox` | The camera's view volume as six planes, and the box test used for culling |
| `renderer/Mesh.nox` | `Vertex` (48 bytes: position, normal, uv, colour) and `MeshData` (vertices and indices) |
| `renderer/Texture.nox`, `Png.nox` | `TextureData` (RGBA8 pixels, filter, wrap), PNG reading and writing |
| `renderer/Primitives.nox` | Cube, box, plane, ramp, capsule, sphere and gem meshes built in code |
| `renderer/GltfLoader.nox` | A `.glb`/`.gltf` file flattened into one `MeshData` |
| `renderer/FontData.nox`, `Text.nox` | An 8x8 bitmap font baked into a 128x48 atlas, and text laid out as overlay quads |
| `renderer/HeadlessRenderer.nox` | A `RenderBackend` with no GPU |
| `sdl-facade/renderer/GpuRenderer.nox` | `GpuRenderer`: the `RenderBackend` over SDL GPU; the frame itself |
| `sdl-facade/renderer/GpuDevice.nox` | Device options, the depth format and the swapchain set-up |
| `sdl-facade/renderer/GpuPipelines.nox` | The eleven pipelines: lit meshes, lines, overlay, shadow, sky and the six full-screen post passes |
| `sdl-facade/renderer/GpuPost.nox` | The post-processing targets (HDR scene, bloom chain, occlusion, the LDR texture for FXAA) and the bloom, occlusion, composite and FXAA passes |
| `sdl-facade/renderer/GpuSky.nox` | The sky's uniform blocks and the sky colours the lit shader reads |
| `sdl-facade/renderer/GpuShadows.nox` | The two shadow-map textures, and the uniform block that tells the shader where each tile came from |
| `sdl-facade/renderer/GpuShaders.nox` | Finding and loading the compiled shaders (SPIR-V or MSL) |
| `sdl-facade/renderer/GpuResources.nox` | GPU meshes and textures, samplers, uploads, mipmaps, growable vertex buffers |
| `sdl-facade/renderer/GpuUniforms.nox` | The uniform blocks, laid out exactly as the shaders declare them |
| `sdl-facade/renderer/GpuDrawList.nox` | The order meshes are drawn in |
| `engine/shaders/*.vert`, `*.frag`, `*.glsl` | The GLSL shaders: `lit`, `shadow`, `sky`, `lines`, `overlay` and the full-screen passes `bloom-downsample`, `bloom-upsample`, `ao`, `ao-blur`, `composite` and `fxaa`, with `fullscreen.vert`, `sky-common.glsl` and `ao-common.glsl` shared. `engine/shaders/compiled/` holds them built for Vulkan and Metal (`mist run build-shaders` rebuilds them) |

Every type, field and function in these files, the shaders, the game loop (`engine/loop/`), the application
(`application/`) and the rest of `sdl-facade/` has a `/** */` doc comment saying what it does and why: the frame order,
who owns what and when it is freed, and the GPU concept behind each call. The language server shows them on hover, in
completion and in signature help. The shaders explain themselves with `//` comments.

## 1. Start-up: device, pipelines, default resources

`GpuRenderer:::Create(window, options)` (`GpuRenderer.nox`) does the one-time work:

1. **Open a device** (`OpenDevice`). The renderer asks for a device that accepts SPIR-V or MSL shaders, since those
   are the two compiled forms it ships, so SDL GPU chooses Vulkan or Metal for the platform. Direct3D 12 would need
   DXIL shaders as well.
2. **Claim the window** for the device, which gives it a swapchain: the images the window shows.
3. **Pick formats** (`GpuDevice.nox`, `GpuPost.nox`). Colour comes from the swapchain. The swapchain is set to SDR_LINEAR where the
   window supports it, so the hardware converts linear colour to sRGB as pixels are written; otherwise the shaders
   do the conversion themselves. Depth takes the most precise format the card has (32-bit float first).
4. **Build the pipelines** (`GpuPipelines.nox`). A pipeline fixes everything about how one kind of geometry is
   drawn: its shaders, its vertex layout, the primitive type, culling, depth testing and blending. There are eleven.
   The first five draw geometry:

   | Pipeline | Draws | Vertex | Primitive | Back-face culling | Depth test / write | Blending | Draws into |
   | --- | --- | --- | --- | --- | --- | --- | --- |
   | `lit` | meshes | 48 bytes: position, normal, uv, colour | triangles | yes | yes / yes | off | the HDR scene texture |
   | `sky` | the sky, where no mesh was drawn | none (one oversized triangle) | triangles | no | yes / no | off | the HDR scene texture |
   | `shadow` | casters, into a depth texture | the lit vertex (only the position is read) | triangles | no | yes / yes, no colour target | off | a shadow map |
   | `lines` | debug lines and particles | 28 bytes: position, colour | lines | no | yes / no | alpha | the screen image |
   | `overlay` | HUD, menus, text | 32 bytes: 2D position, uv, colour | triangles | no | no / no | alpha | the screen image |

   The other six are full-screen post passes. They take no vertex buffer: `fullscreen.vert` makes one oversized
   triangle that covers the target, so the fragment shader runs once per pixel.

   | Pipeline | Reads | Draws into |
   | --- | --- | --- |
   | `bloom-downsample` | the scene, then each smaller bloom level | the next bloom level (HDR float) |
   | `bloom-upsample` | a bloom level | the next larger level, added on (additive blend) |
   | `ao` | the depth buffer | the raw occlusion texture (8-bit, one channel) |
   | `ao-blur` | the raw occlusion and the depth buffer | the blurred occlusion texture |
   | `composite` | the scene, the bloom, the occlusion and the depth buffer | the screen image, or the LDR texture |
   | `fxaa` | the LDR texture | the screen image |

   The `lit` and `sky` pipelines draw into the 16-bit float scene texture, so they are built for that format. Each
   pipeline's shaders are loaded from `engine/shaders/compiled/` (`GpuShaders.nox`): `.spv` for Vulkan, `.msl` for
   Metal.
5. **Make the samplers and the white texture** (`GpuResources.nox`). There are four samplers (linear or nearest,
   repeat or clamp) and every texture uses the one matching its settings. A 1x1 white texture stands in wherever a
   draw has no texture, so the shader can always sample one and multiply.

`GpuRenderer:::CreateOffscreen` does the same without a window; the editor's viewport and the GPU tests use it.

## 2. Long-lived resources: meshes and textures

A game uploads a mesh or texture once and refers to it by handle afterwards.

- `CreateMesh(MeshData)` validates the indices, creates a vertex buffer and an index buffer on the GPU, copies the
  data through a *transfer buffer* (memory the CPU can write and the GPU can read) in a copy pass, and keeps the
  mesh's bounding box for culling (`UploadMesh`).
- `CreateTexture(TextureData)` uploads the pixels the same way into an sRGB texture. A texture with linear filtering
  also gets a full mipmap chain generated on the GPU, so it doesn't shimmer when seen from far away
  (`UploadTexture`, `MipLevelCount`).
- Both live in a `Pool`, which hands back a *generational handle*: an index and a generation number. Removing a mesh
  bumps its slot's generation, so an old handle stops matching. A draw that still holds it is skipped with a warning
  instead of reading freed memory (`core/Handle.nox`).

## 3. The game fills a RenderWorld

Each frame, before anything touches the GPU, the game describes the frame (`host/GameHost.nox`, `Render`):

1. `renderWorld->Clear()` empties last frame's draws, lines and overlay. The lights, sky, fog and post settings stay as
   the game last set them, so a game sets them once, not every frame.
2. The game's `OnRender` hook sets the camera, and may add draws, lines, blob shadows, particles and overlay text
   directly.
3. `StepParticles` moves the particles on by the frame's time and adds a line streak for each to the frame's lines.
4. The game's UI draws its boxes and text as overlay quads.
5. `ExtractDraws` (`engine/entities/World.nox`) walks every entity with a visible mesh renderer and adds a
   `DrawItem` for it: mesh handle, texture handle, world transform, tint colour, material and normal map. The transform
   is interpolated between the last two simulation steps with the loop's `alpha`, which is what keeps motion smooth when
   the display rate and the 60 Hz simulation don't line up (see `application-and-game-loop.md`).
6. A script error, if any, and the debug HUD go on the overlay last, so they're on top.
7. `renderer->Render(renderWorld)`.

A `RenderWorld` holds only plain values and handles, never GPU objects. That is what lets the same frame go to
`GpuRenderer`, `HeadlessRenderer` or `Capture` unchanged.

## 4. Render: one frame on the GPU

`GpuRenderer.Render` (`GpuRenderer.nox`):

1. **Acquire a command buffer.** GPU work is recorded into a command buffer on the CPU and submitted all at once.
2. **Upload this frame's dynamic geometry** (`UploadDynamicGeometry`). Lines and overlay vertices change every frame,
   so they're copied into two growable vertex buffers in a copy pass. The buffers only grow, so a steady frame does
   no allocation.
3. **Acquire the swapchain image** to draw into. If there is none (the window is minimised), submit the empty
   command buffer and report `presented: false`.
4. **Make sure the depth buffer matches** the image's size; it's recreated when the window is resized
   (`EnsureDepthTarget`).
5. **Record the frame** (`RecordFrame`, below).
6. **Submit.** SDL presents the image once the GPU has finished with it.

### RecordFrame

1. **Build the camera matrices.** The aspect ratio comes from the target's size. `Camera.ViewProjection` multiplies
   the projection (perspective, or orthographic for the editor's straight views) by the view (a right-handed
   look-to matrix). Lantern's axes: +Y up, the camera looks down -Z, and clip-space depth runs 0 to 1.
2. **Cull** (`CollectVisibleDraws`). The view-projection matrix gives the frustum's six planes (`Frustum.nox`, the
   Gribb and Hartmann method). Each draw's mesh bounds are moved by its transform and tested against them: a box
   wholly outside any plane is skipped and counted in `drawsCulled`.
3. **Sort** (`GpuDrawList.nox`). The visible draws are sorted by texture, then mesh, then submission order, so
   consecutive draws share bindings.
4. **Draw the shadow maps** (`RecordShadowPasses`, see "Shadows" below), in passes of their own before the main one.
5. **Make sure the post targets match** the frame's size (`EnsurePostTargets`): the high dynamic range scene texture,
   the occlusion textures, the LDR texture FXAA reads, and the bloom chain.
6. **Scene pass.** One render pass with the scene texture cleared to the sky's horizon colour (or the clear colour when
   the sky is off) and depth cleared to 1.0, the far plane. The scene texture is 16-bit float per channel (RGBA16F, which
   the GPU must be able to render to; see `graphics-effects.md`), so light brighter than 1 is kept as it is and not cut
   off. Then **draw the meshes** (`DrawMeshes`). Push the per-frame uniforms once: the view-projection matrix and camera
   position for the vertex shader, and the sun, the ambient light, the sky's colours, fog and the chosen local lights for
   the fragment shader. Then for each draw, bind its mesh's buffers and its texture only if they differ from the previous
   draw's, push the per-draw uniforms (the model matrix and its normal matrix, then the tint and material) and issue one
   indexed draw. After the meshes, **draw the sky** (`DrawSky`, if the sky is on and the camera is perspective): a
   fullscreen triangle at the far depth that fills only the pixels no mesh covered. The same sky function lights the
   meshes. See `sky-and-image-based-lighting.md`. The pass keeps its depth, because the occlusion and composite passes
   and the lines read it next.
7. **Bloom** (`RecordBloom`), if it's on: shrink the scene through five textures, each half the size of the last (the
   first is half the frame), keeping only the bright part in the first shrink, then grow them back one at a time adding
   each to the next larger. Every step is a full-screen pass. See `bloom-and-tone-mapping.md`.
8. **Ambient occlusion** (`RecordAmbientOcclusion`), if it's on and the camera is perspective: read the scene's depth
   buffer to estimate how blocked each pixel is, then average that over 4x4 pixels. See `ambient-occlusion.md`.
9. **Composite** (`RecordComposite`): a full-screen pass that reads the scene, the bloom, the occlusion and the depth
   buffer. In this order it multiplies in the occlusion, darkens under blob shadows (`blob-shadows.md`), adds height fog
   and light shafts (`fog-and-light-shafts.md`), adds the bloom, multiplies by the exposure, applies the tone-mapping curve
   and the colour grade (`colour-grade.md`), and, when the target can't sRGB-encode, does that last. It writes the
   swapchain image, or, with anti-aliasing on, a texture of its own that **FXAA** (`RecordAntiAlias`) then smooths onto
   the screen image (`anti-aliasing.md`).
10. **Overlay pass**, only if there are lines or overlay quads: it loads the screen image and the scene's depth, then
    **draws the lines** (`DrawLines`), depth-tested against the meshes but not writing depth, and **the overlay**
    (`DrawOverlay`) in screen pixels, with no depth test, so it sits on top. Particles are lines, so they are drawn here
    (`particles.md`). Every overlay vertex samples the font atlas; solid boxes use the atlas's white cell. Lines and
    overlay come after the composite on purpose: a HUD colour or a debug line's colour reaches the screen exactly as
    written, not tone mapped.

A normal map (`normal-maps.md`) is bound beside the draw's texture in step 6 and bends the normal in the lit shader.

### The uniforms

Uniform blocks are small sets of constants pushed before a draw. `GpuUniforms.nox` lays each one out exactly as the
GLSL declares it (std140, so every field is a whole number of 16-byte rows):

| Stage, slot | Block | Pushed | Holds |
| --- | --- | --- | --- |
| vertex 0 | `MatrixAndVectorUniforms` (80 bytes) | once per frame | view-projection, camera position |
| vertex 1 | `ObjectUniforms` (128 bytes) | per draw | model matrix, normal matrix |
| fragment 0 | `LightUniforms` (176 bytes) | once per frame | direction to the sun, sun colour, sky and ground ambient, fog colour and range, the sRGB flag, camera position, the sky's colours and how strongly it lights the scene |
| fragment 1 | `MaterialUniforms` (48 bytes) | per draw | tint, metallic and roughness, normal map strength, emissive colour |
| fragment 2 | `LocalLightUniforms` (528 bytes) | once per frame | how many local lights, then up to eight of them (position, range, colour, kind, direction, cone cosines, first shadow tile) |
| fragment 3 | `ShadowUniforms` (1024 bytes) | once per frame | each shadow tile's light matrix, where each sun cascade ends, texel sizes, the camera's forward vector, the depth bias, the filter's softness and sample count, the cascade blend |

The lit fragment shader's samplers are the draw's texture in slot 0, the sun's and the local lights' shadow maps in
slots 1 and 2, and the draw's normal map in slot 3 (a 1 by 1 flat map when it has none).

The post passes have blocks of their own, each in fragment slot 0: the sky's `SkyUniforms` (128 bytes: the camera's
axes, the sun's direction and disc, the three sky colours), the bloom shrink's threshold (16 bytes), the occlusion's
`OcclusionUniforms` (48 bytes: the camera's projection terms, the radius, intensity and bias) and the composite's
`CompositeUniforms` (464 bytes: which effects are on, the camera, the fog, the light shafts, the blob shadows, the
exposure, the tone mapper and the colour grade).

The normal matrix is the inverse transpose of the model matrix's 3x3 part. Normals moved by the model matrix itself
would tilt off their surface under non-uniform scale.

### The shaders

`lit.vert` moves each vertex to world space with the model matrix and to clip space with the view-projection matrix.
`lit.frag` computes, for each pixel:

```text
albedo   = texture(uv) * vertex colour * tint
normal   = the mesh's normal, bent by the normal map when the draw has one
flat     = mix(groundAmbient, skyAmbient, 0.5 + 0.5 * normal.y) * (diffuseColour + f0 * (1 - roughness))
sky      = (the sky around the normal * diffuseColour + the sky in the mirror direction * its reflectance) * intensity
colour   = mix(flat, sky, skyStrength)                                 the sky lights the scene, or the flat pair does
colour  += EvaluateLight(sun) * sunShadow
colour  += EvaluateLight(light) * attenuation(light, pixel) * shadow   for each of up to eight point and spot lights
colour  += emissive
colour   = mix(colour, fogColour, fog)                                 the linear fog between the near and far distances
```

and writes the result as linear light, unclamped, into the scene texture; `composite.frag` adds the post effects, tone maps
it and encodes it to sRGB afterwards. `lines` passes colours through. `overlay.vert` turns pixel
positions into clip space (`x / width * 2 - 1`, `1 - y / height * 2`).

`EvaluateLight` is the Cook-Torrance model, the usual "PBR-lite" shading:

- **Diffuse**: the albedo, for non-metals, times how squarely the surface faces the light (`dot(normal, toLight)`).
  A metal has no diffuse light: `diffuseColour = albedo * (1 - metallic)`.
- **Specular**: a highlight shaped by three terms. GGX says how many of the surface's microscopic mirrors face
  half-way between the light and the eye (rougher surfaces spread it out). Smith visibility says how many of them are
  hidden by their neighbours. Schlick's Fresnel says surfaces reflect more at grazing angles. A non-metal reflects
  about 4% of the light (`f0 = 0.04`); a metal reflects its own colour (`f0 = albedo`).
- **Light units**: a light of strength 1 facing a white matte surface gives colour 1. The textbook formulas divide
  diffuse light by pi, so `EvaluateLight` multiplies by pi to cancel it. That keeps the numbers an artist writes
  meaning what they look like.

A point light's strength falls with the square of the distance and is windowed to exactly zero at its `range`
(`DistanceAttenuation`), so the shader can skip it beyond that. A spot light also fades from full strength at its
inner cone to nothing at its outer cone with a `smoothstep` on the cosine of the angle to its axis; the cosines are
worked out on the CPU in `PackLocalLights`, not per pixel.

Only eight local lights are lit per frame (`MAX_LOCAL_LIGHTS`). When a scene has more, `SelectLocalLights` takes the
eight whose reach is nearest the camera, so lights come and go as the player walks past them. Beyond a handful of
lights the usual answer is clustered or deferred shading, which this renderer leaves out.

## Shadows

Light that nothing blocks isn't shadowed; the renderer finds out what blocks it by looking at the scene from each
shadowing light. Each such picture, made of depths and not colours, is a *shadow map*. Before the main pass the
renderer draws two:

| Map | Holds | Size by default |
| --- | --- | --- |
| sun | three cascades side by side | 3 x 1024 by 1024 |
| local | a grid of tiles, 6 across and 2 down: one for each shadowed spot light and six for each shadowed point light | 6 x 512 by 2 x 512 |

Each is one depth texture drawn in one render pass, with the viewport and scissor moved to each tile in turn
(`RecordShadowMap`). The `shadow` pipeline draws every shadow-casting mesh whose bounds meet the tile's view volume
(`RecordShadowTile`), with the vertex shader `shadow.vert` placing each vertex and an empty `shadow.frag`: the depth
the GPU computes while rasterising is the whole output.

If the GPU refuses to create a shadow map, the frame is drawn without shadows and a warning is logged. The lit shader still needs something bound at each shadow slot, so `ShadowTargets` keeps a 1 by 1 fallback depth texture for that case.

**Planning** (`renderer/ShadowPlan.nox`) is pure data and runs first, so a test can check it with no GPU.

- **The sun's cascades.** A sun is infinitely far away, so its shadow map is orthographic. One map over the whole view
  would give each texel a fistful of metres, so the camera's view is cut by distance into three slices (halfway
  between an even and a logarithmic split) and each gets its own map. The player's feet get the finest texels and
  distant hills the coarsest. Each cascade's map is just big enough to hold a sphere around its slice. The sphere
  doesn't change size as the camera turns, and the box's position is rounded to whole texels, so shadow edges don't
  crawl and shimmer as the player moves. `ShadowSettings.distance` is where the last cascade ends: beyond it nothing
  is shadowed.
- **Spot lights** get one perspective tile looking down the cone, a little wider than the cone itself.
- **Point lights** shine every way, so each gets six tiles, one per face of a cube around it (+X, -X, +Y, -Y, +Z, -Z),
  each opening 95 degrees: a little more than the 90 that tiles a cube, so the filter at a face's edge still has depth
  to read. The shader picks the face from the larger axis of the vector from the light to the pixel.
- **Who gets tiles.** Only the lights `SelectLocalLights` chose for the frame, in that order, until the twelve tiles
  are used: a spot light takes one and a point light six. A light with `castsShadows` off costs nothing.

**Looking up** happens in `lit.frag`. For each pixel the shader transforms its position by the tile's light matrix and
compares the result's depth with the depth stored in the map. If the pixel is nearer, the light reaches it; if farther,
something stands between. That comparison is the shadow sampler's job: a sampler made with a compare operation takes a
reference depth, compares it with the stored one and blends the answers of the four nearest texels. The shader repeats that lookup
at 4, 12 or 20 points (the quality level) spread over a disc and rotated differently in each pixel, which fades an edge
over a few texels (`FilteredShadow`; see `shadow-softness-and-cascade-blending.md`). With cascade blending on, the last part of
each sun cascade also looks up in the next one and mixes the two. The lookups are fenced inside the pixel's own tile so
they never read a neighbour's depths.

**Acne** is the speckle a surface gets when it shadows itself, because a map's texels are coarser than the surface and
some of them hold a depth slightly nearer than the surface really is. Two biases keep it away:

- *Slope bias*, in the shadow pipeline: while drawing the map the rasteriser pushes depths farther by an amount
  that grows with how steeply the triangle tilts from the light (`depth_bias_constant_factor` and
  `depth_bias_slope_factor`).
- *Normal offset*, in the shader: before the lookup a pixel moves along its own surface normal by a bit more than a
  texel (`SunShadow`, `LocalShadow`). A surface that faces the light moves toward it and clears the map's depths. The
  push is in world units: a sun texel is a fixed size, and a perspective map's texels grow with distance from the
  light, which is why the plan carries a texel size for each.

A constant depth bias (`SUN_DEPTH_BIAS`) covers what's left. Too much bias detaches a shadow from its caster
("peter-panning"), so these are small, and the numbers are the first thing to change if a new scene shows acne.

**Cost.** A tile is one more pass over the casters inside its view volume, so a frame draws roughly (casters near the
camera) x (three cascades) for the sun and as many again for each shadowed light. That is why a spot light is cheap
and a point light is six times that, why only twelve tiles exist, and why lights default to no shadows. Smaller
`ShadowSettings` sizes and a shorter `distance` are the first things to turn down.

## Quality levels and cost

`lantern:::graphics:::SetQuality("low" | "medium" | "high")` is the one switch for how much the renderer spends
(`quality-settings.md` is the guide for choosing). It sets each effect's on or off state and a few counts, in
`PostSettings.WithQuality`, `AtmosphereSettings.WithQuality`, `ShadowSettings.WithQuality` and the particle budget:

| | low | medium (default) | high |
| --- | --- | --- | --- |
| Bloom | off | on | on |
| Anti-aliasing (FXAA) | off | on | on |
| Ambient occlusion | off | on | on |
| Light shafts | off | on | on |
| Screen-space reflections | off | on | on |
| Shadow filter lookups per pixel | 4 | 12 | 20 |
| Particle budget | 64 | 256 | 512 |

What each effect adds to a frame, so you know what to turn down first. These are counts of work, not measured times: the
cloud machine has only a software Vulkan driver, so frame times on a real GPU and Metal output are still to be checked
(`known-gaps.md`).

| Effect | Extra GPU work | Changed by quality |
| --- | --- | --- |
| Shadow maps | One depth pass per tile: up to 3 sun cascades and 12 local tiles, each redrawing the casters inside it | the filter's lookups, not the tile count; `SetShadowQuality` sets map sizes and distance |
| Sky and sky lighting | One triangle, and two sky-colour evaluations per lit pixel | no |
| Bloom | 9 full-screen passes (5 shrinks, 4 growths) at half the frame's size and below | on or off |
| Ambient occlusion | 2 full-screen passes on frame-sized one-channel textures: 12 depth reads per pixel to estimate, 16 to blur. The costliest effect | on or off |
| Composite | 1 full-screen pass; height fog and the grade are a few arithmetic operations, blob shadows a loop of at most 8, shafts 24 depth reads per pixel | shafts on or off |
| FXAA | 1 full-screen pass and one extra frame-sized texture | on or off |
| Particles | One line per live particle, in the overlay pass | the budget |
| Normal maps | One texture sample per pixel of a draw that has a map | no |

With reflections on there is one more full-screen pass between the scene and bloom (`screen-space-reflections.md`). At the top quality a frame is therefore up to 18 render passes beyond the shadow tiles (two shadow maps, the scene, nine
bloom, two occlusion, the reflections, the composite, FXAA and the overlay); at Low it is the shadow maps, the scene, the composite and
the overlay. Turn effects down in this order when a frame is too slow: ambient occlusion, then bloom, then shadow
quality, then FXAA.

## 5. Capture and off-screen rendering

`Capture(world, width, height)` records the same frame, every pass above included, into a texture of its own instead of the swapchain, copies
the texture into a download transfer buffer, waits on a fence for the GPU to finish, and copies the pixels back into
a `TextureData`. On drivers whose images are blue-green-red, it swaps red and blue. Screenshots
(`LANTERN_CAPTURE_PATH`) and the GPU tests use it. `RenderOffscreen` is the same with a target kept between frames,
for the editor's viewport.

## 6. Shutdown

`ReleaseDevice` waits for the GPU to go idle, then releases every mesh, texture, buffer, sampler and pipeline, gives
the window back and destroys the device. It must run before the window is destroyed and before SDL shuts down, which
is why the application deletes the game (which owns the renderer) before itself (see
`application-and-game-loop.md`).

## What this renderer leaves out, and where it would go

The renderer is small because it does one forward pass with a handful of lights, a few shadow maps and a short chain of
full-screen passes, and SDL GPU handles the device, swapchain, memory and synchronisation underneath. These are the
usual next features, and where each would fit:

| Feature | What changes |
| --- | --- |
| Decals | A projected decal needs the depth buffer read in the lit pass, or a pass of its own after the scene pass, and a texture bound to it. A flat quad drawn just above a surface covers flat ground meanwhile (`normal-maps.md`). |
| A true penumbra | Shadows widen with the distance from the caster. That needs raw depths in the shader, which the comparison sampler can't give, so it would read the map with a plain sampler (`shadow-softness-and-cascade-blending.md`). |
| Caching shadow maps | Maps of lights that don't move need per-light dirty flags in `RenderWorld`. |
| Richer materials | `Material` has metallic, roughness and emissive values. Roughness and emissive *textures*, and per-material shaders, would need a material handle on `DrawItem` instead of one texture, and a draw list that sorts by pipeline, then material, then mesh. |
| Many lights | Eight lights in a loop is the limit of a forward renderer with a uniform block. A storage buffer and a clustered or deferred approach go past it. |
| Transparency | A second list in `RenderWorld` for blended draws, sorted back to front and drawn after the opaque meshes with depth writes off. |
| Depth of field, motion blur, temporal anti-aliasing | Each is another pass between the scene pass and the composite, or a step inside it, like bloom and occlusion; temporal anti-aliasing also needs motion vectors and the previous frame. |
| Billboard particles | A pipeline of its own, with a texture, additive blending and soft edges (`particles.md`). |
| Instancing | Group draws that share a mesh and material, put their transforms in a buffer, and issue one draw with an instance count. `GpuDrawList.nox` already sorts them together. |
| A render graph | There are now enough passes that describing each one's inputs and outputs and letting a small scheduler order them and manage their textures would replace the hand-written `RecordFrame`. |

Each of these lives behind `RenderBackend`: the game keeps filling a `RenderWorld`, and only `renderer/` (new
vocabulary) and the backend (new GPU work) change.
