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
                                     light, fog)                  └─ HeadlessRenderer (tests)
```

Two ideas carry the whole design:

1. **The frame is data.** Nothing above the renderer calls a GPU function. Each frame, the game fills a
   `RenderWorld`: a list of draw items, a list of debug lines, a list of 2D overlay vertices, and the camera, light,
   clear colour and fog. The renderer reads it and forgets it.
2. **One small interface between "what" and "how".** `RenderBackend` (`engine/src/renderer/RenderBackend.nox`) has
   seven methods: its driver name, create and remove a mesh, create and remove a texture, render a frame, and
   capture a frame to memory. `GpuRenderer` implements it over SDL GPU; `HeadlessRenderer` implements it with no
   GPU at all, for tests. A Vulkan or Direct3D renderer written by hand would be a third implementation, and nothing
   above the interface would change.

## The files

| File | What it holds |
| --- | --- |
| `renderer/RenderBackend.nox` | The `RenderBackend` interface and `RenderReport` |
| `renderer/RenderWorld.nox` | `RenderWorld` (one frame), `DrawItem`, `Material`, `DirectionalLight` (the sun), `Light` (point and spot), `LineVertex`, the 2D `Overlay` |
| `renderer/ShadowPlan.nox` | `ShadowPlan`: which shadow tiles to draw and from where (the sun's cascades, each shadowed light's tiles), as plain data |
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
| `sdl-facade/renderer/GpuPipelines.nox` | The four pipelines: lit meshes, lines, overlay, shadow |
| `sdl-facade/renderer/GpuShadows.nox` | The two shadow-map textures, and the uniform block that tells the shader where each tile came from |
| `sdl-facade/renderer/GpuShaders.nox` | Finding and loading the compiled shaders (SPIR-V or MSL) |
| `sdl-facade/renderer/GpuResources.nox` | GPU meshes and textures, samplers, uploads, mipmaps, growable vertex buffers |
| `sdl-facade/renderer/GpuUniforms.nox` | The uniform blocks, laid out exactly as the shaders declare them |
| `sdl-facade/renderer/GpuDrawList.nox` | The order meshes are drawn in |
| `engine/shaders/*.vert`, `*.frag` | The GLSL shaders; `engine/shaders/compiled/` holds them built for Vulkan and Metal |

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
3. **Pick formats** (`GpuDevice.nox`). Colour comes from the swapchain. The swapchain is set to SDR_LINEAR where the
   window supports it, so the hardware converts linear colour to sRGB as pixels are written; otherwise the shaders
   do the conversion themselves. Depth takes the most precise format the card has (32-bit float first).
4. **Build four pipelines** (`GpuPipelines.nox`). A pipeline fixes everything about how one kind of geometry is
   drawn: its shaders, its vertex layout, the primitive type, culling, depth testing and blending.

   | Pipeline | Draws | Vertex | Primitive | Back-face culling | Depth test / write | Blending |
   | --- | --- | --- | --- | --- | --- | --- |
   | `lit` | meshes | 48 bytes: position, normal, uv, colour | triangles | yes | yes / yes | off |
   | `lines` | debug lines | 28 bytes: position, colour | lines | no | yes / no | alpha |
   | `overlay` | HUD, menus, text | 32 bytes: 2D position, uv, colour | triangles | no | no / no | alpha |
   | `shadow` | casters, into a depth texture | the lit vertex (only the position is read) | triangles | no | yes / yes, no colour target | off |

   Each pipeline's shaders are loaded from `engine/shaders/compiled/` (`GpuShaders.nox`): `.spv` for Vulkan, `.msl`
   for Metal.
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

1. `renderWorld->Clear()` empties last frame's lists.
2. The game's `OnRender` hook sets the camera, light and fog, and may add draws, lines and overlay text directly.
3. The game's UI draws its boxes and text as overlay quads.
4. `ExtractDraws` (`engine/entities/World.nox`) walks every entity with a visible mesh renderer and adds a
   `DrawItem` for it: mesh handle, texture handle, world transform and tint colour. The transform is interpolated
   between the last two simulation steps with the loop's `alpha`, which is what keeps motion smooth when the display
   rate and the 60 Hz simulation don't line up (see `application-and-game-loop.md`).
5. The debug HUD adds its panel to the overlay.
6. `renderer->Render(renderWorld)`.

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
5. **Begin one render pass** with the colour image cleared to the world's clear colour and depth cleared to 1.0, the
   far plane.
6. **Draw the meshes** (`DrawMeshes`). Push the per-frame uniforms once: the view-projection matrix and camera
   position for the vertex shader, and the sun, the ambient light, fog and the chosen local lights for the fragment
   shader. Then for each draw, bind its mesh's buffers and its texture only if they differ from the previous draw's,
   push the per-draw uniforms (the model matrix and its normal matrix, then the tint and material) and issue one
   indexed draw.
7. **Draw the lines** (`DrawLines`), depth-tested against the meshes but not writing depth.
8. **Draw the overlay** (`DrawOverlay`) in screen pixels, with no depth test, so it sits on top. Every overlay
   vertex samples the font atlas; solid boxes use the atlas's white cell.
9. **End the pass.**

### The uniforms

Uniform blocks are small sets of constants pushed before a draw. `GpuUniforms.nox` lays each one out exactly as the
GLSL declares it (std140, so every field is a whole number of 16-byte rows):

| Stage, slot | Block | Pushed | Holds |
| --- | --- | --- | --- |
| vertex 0 | `MatrixAndVectorUniforms` (80 bytes) | once per frame | view-projection, camera position |
| vertex 1 | `ObjectUniforms` (128 bytes) | per draw | model matrix, normal matrix |
| fragment 0 | `LightUniforms` (112 bytes) | once per frame | direction to the sun, sun colour, sky and ground ambient, fog colour, fog range and the sRGB flag, camera position |
| fragment 1 | `MaterialUniforms` (48 bytes) | per draw | tint, metallic and roughness, emissive colour |
| fragment 2 | `LocalLightUniforms` (528 bytes) | once per frame | how many local lights, then up to eight of them (position, range, colour, kind, direction, cone cosines, first shadow tile) |
| fragment 3 | `ShadowUniforms` (1008 bytes) | once per frame | each shadow tile's light matrix, where each sun cascade ends, texel sizes, the camera's forward vector, the depth bias |

The fragment shader's samplers are the draw's texture in slot 0 and the two shadow maps in slots 1 and 2.

The normal matrix is the inverse transpose of the model matrix's 3x3 part. Normals moved by the model matrix itself
would tilt off their surface under non-uniform scale.

### The shaders

`lit.vert` moves each vertex to world space with the model matrix and to clip space with the view-projection matrix.
`lit.frag` computes, for each pixel:

```text
albedo   = texture(uv) * vertex colour * tint
ambient  = mix(groundAmbient, skyAmbient, 0.5 + 0.5 * normal.y)       faces up: sky light, faces down: ground light
colour   = ambient * (diffuseColour + f0 * (1 - roughness))
colour  += EvaluateLight(sun)
colour  += EvaluateLight(light) * attenuation(light, pixel)           for each of up to eight point and spot lights
colour  += emissive
colour   = mix(colour, fogColour, fog)                                fog fades into the clear colour, hiding the far plane
```

and encodes to sRGB itself when the swapchain can't. `lines` passes colours through. `overlay.vert` turns pixel
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
reference depth, compares it with the stored one and blends the answers of the four nearest texels. The shader adds a
3x3 loop of those, which fades an edge over about three texels (`FilteredShadow`). The lookups are fenced inside
the pixel's own tile so they never read a neighbour's depths.

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

## 5. Capture and off-screen rendering

`Capture(world, width, height)` records the same frame into a texture of its own instead of the swapchain, copies
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

The renderer is small because it does one forward pass with a handful of lights and a few shadow maps, and SDL GPU handles the device, swapchain,
memory and synchronisation underneath. These are the usual next features, and where each would fit:

| Feature | What changes |
| --- | --- |
| Softer, cheaper shadows | Cascade blending, percentage-closer soft shadows (a filter that widens with the distance to the caster) and caching maps of lights that don't move would each be a change to `lit.frag` and `ShadowPlan.nox`; a shadow-caching scheme needs per-light dirty flags in `RenderWorld`. |
| Richer materials | `Material` has metallic, roughness and emissive values. Normal, roughness and emissive *textures*, and per-material shaders, would need a material handle on `DrawItem` instead of one texture, and a draw list that sorts by pipeline, then material, then mesh. |
| Many lights | Eight lights in a loop is the limit of a forward renderer with a uniform block. A storage buffer and a clustered or deferred approach go past it. |
| Transparency | A second list in `RenderWorld` for blended draws, sorted back to front and drawn after the opaque meshes with depth writes off. |
| Post-processing | Render the scene into an off-screen colour texture (as `Capture` does), then draw a full-screen triangle that samples it (tone mapping, bloom, anti-aliasing) into the swapchain. |
| Instancing | Group draws that share a mesh and material, put their transforms in a buffer, and issue one draw with an instance count. `GpuDrawList.nox` already sorts them together. |
| A render graph | Once there are several passes, describe each pass's inputs and outputs and let a small scheduler order them and manage their textures, instead of a hand-written `RecordFrame`. |

Each of these lives behind `RenderBackend`: the game keeps filling a `RenderWorld`, and only `renderer/` (new
vocabulary) and the backend (new GPU work) change.
