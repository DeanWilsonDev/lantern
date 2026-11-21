# Gameplay Direction: Finding Her

Related: [`crimson-and-thorn-slice.md`](crimson-and-thorn-slice.md), [`gameplay-prototypes.md`](gameplay-prototypes.md),
[`grounded-adventure.md`](grounded-adventure.md), [`finding-her-prototypes.md`](finding-her-prototypes.md) (what is built).

---

> **Status: exploration, updated 2026-11-21.** Nothing here replaces the other design docs yet. The first version of this page offered three directions inside a 3D platformer. After reading it, Dean leaned away from a platformer entirely and towards **searching as the core mechanic**, because searching changes how it works once she is gone. This version designs that game: the search loop, how it changes over the chapters, a tightly capped budget of one small vignette per chapter, the camera perspective, and scope. The earlier directions are summarised at the end.

## The problem

Jumping about and solving puzzles feels too whimsical for the story. Without platforming, though, the obvious question is what the player actually *does*: combat and driving make no sense, and walking alone isn't fun.

The answer on this page: **walking is how you move, not what you do. What you do is look for her.** Searching is a real loop, with information to read, choices to make and soft pressure, and it is the one mechanic in the game that turns into grief by itself: the same inputs, with nobody left to find.

What Dean wants, and this design keeps:

- A weighty, grounded character with the feel of Red Dead Redemption 2's movement, **without** the realism scope.
- The beat he loves: moving through crowds together, losing her, finding her again, and coming back to that after she's gone.
- Small scope. Per-scene mechanics are allowed only if they are tiny and built from parts the game already has.

## What kind of game this is

A **third-person narrative adventure**, not a platformer. You walk through authored Tokyo spaces with her, you lose her, you search for her, and you find her. Over five chapters, the finding gets harder, then hollow, then impossible.

- **No free jump.** A contextual step, hop and mantle onto curbs, benches and low walls, used to get a better view, not to cross gaps.
- **No puzzles in the lock-and-key sense.** The challenge is reading a place to find a person.
- **No fail states.** You can never be lost in a way that stops the game. Her answer always gives a direction.

If Dean picks this, the genre line in the Crimson and Thorn design and the game design document would change from "3D narrative puzzle platformer" to "3D narrative adventure". That is not done here.

---

## The core loop

Every level is built from the same four-beat cycle. The chapters change how long each beat lasts and what happens in it.

| Beat | What happens | What the player does |
|---|---|---|
| **Together** | You walk beside her. She notices things, steers you towards what she likes, maybe takes your hand. | Walk, hold hands, look where she looks. This is also when you *learn her*: which stalls she stops at, what she points at, where she likes to stand. |
| **Separated** | A crowd pulls you apart, she wanders off, a train door closes, the fog comes in. Always authored, never random. | Nothing yet. A moment of the camera tightening and the sound changing. |
| **Searching** | She is somewhere in this space. | Call, listen, look, find a high spot, follow her traces, and remember where she likes to go. |
| **Found** | You reach her. A reunion that changes with the chapter: in chapter 2 she runs to you; by chapter 4 she barely turns round. | Walk to her. Take her hand, if she offers it. |

### Why searching is engaging moment to moment

Searching is fun when it is about **information**, not about wandering. The player always has a few tools, each giving a partial answer, and every place weakens one or two of them. Finding her is putting the partial answers together.

| Tool | Input | What it tells you | Built from |
|---|---|---|---|
| **Call** | Tap a button. He calls out (a short wordless sound). | She answers with a sound placed in 3D, and a brief pulse of her glow visible through crowds and walls. A **direction**, roughly. | One input, one positioned sound, one light pulse |
| **Look** | Hold a button. He stops and looks; the camera eases in; the sound of the crowd ducks a little. | Her glow and ponytail stand out a little more; nearby traces catch the light. **Detail** close by. | Look-at layer, a camera nudge, a mix snapshot |
| **Get up high** | Step onto a bench, a railing, a stall's crate, a stair landing. | You see over the crowd. **Distance and position.** | Hop and mantle (built), authored vantage spots |
| **Follow traces** | Walk. | Petals she dropped, glow left on a wall, crowd figures turning their heads to where she went. **A trail.** | Particles, lights, the look-at layer on crowd figures |
| **Know her** | Nothing. It's memory. | She is likely at the takoyaki stall she stopped at in chapter 2, or by the cat outside the bar. **A guess.** | Level design: the Together beats teach it |

What weakens each tool, so each search feels different:

| Situation | What it does | Which tools matter more |
|---|---|---|
| **Dense crowd** (festival, crossing, station) | Blocks sight lines | Call, getting up high |
| **Noise** (fireworks, station announcements, rain on umbrellas) | Masks her answer | Look, traces, getting up high |
| **Fog** (Hakone) | Blurs light and distance | Call (but it echoes), traces underfoot |
| **Darkness** (night streets, late chapters) | Her glow is the only light | Look, her glow; later, the lantern |
| **Moving crowd** (rush hour, the end of the festival) | She keeps getting carried away | Calling more often, keeping close |
| **Time pressure** (last train, fireworks about to start) | Soft, never a fail | Everything; a reason to hurry |

That gives a small matrix of tools and situations. Each search combines one place with one or two situations, so a dozen searches can all feel different without any new mechanics.

He **walks** by default and only **jogs while she is lost**. The single time the player hurries is when she's missing, so the pace itself carries meaning.

---

## How it changes across the chapters

The tools never change. What they find does.

| Chapter | Separations | How long a search takes | Her answer to the call | Traces | Found |
|---|---|---|---|---|---|
| **1 Stability** | None. You are alone. You search for *places* (the temple, by its bell). | Short | Nothing answers. The call is a lonely habit. At the end of the chapter, someone answers for the first time: that is how you meet her. | None | You find her because she answered |
| **2 Warmth** | Playful. She hides in the festival crowd, ducks behind a stall, runs ahead laughing. | Seconds | Instant, close, warm, a bright glow pulse | Fresh petals, bright glow | She runs to you |
| **3 Tension** | She goes on without noticing you're gone. | Longer | Late, further away, sometimes echoing from the wrong direction (fog) | Thinner. Her glow on walls fades faster | She's waiting, but somewhere else in her thoughts |
| **4 Disconnection** | You find her, and she leaves again before you reach her. | Long, and some searches never end in her | Faint. In the Rose Garden, the first time no answer comes | Old: wilted petals, cold glow | Rarely. The chapter ends with her gone |
| **5 Isolation** | There is no one to lose. | As long as you keep searching | Nothing. The city's sound fills the gap | Only memories: the stall, the seat on the train, the place she liked to stand | The level ends when you **stop calling** and stand still while the crowd passes |
| **Blue door runs** | Her ghost is in the crowd. | Short | The ghost answers, but it is the chapter 2 answer, the same sound from the same place every time. It fades each run. | None | You reach the spot and she's not there |

**Know her** is the tool that hurts most in chapter 5. Everything you learned about her in the Together beats still works as a search strategy: you go to the stall, to the cat, to the bench. You know exactly where she'd be. She isn't.

The Two Doors fit with no new design: in the dark room you call, and the ghost answers from beyond the blue door. The red door is the one direction the call doesn't come from.

### Where Boost, Light and Anchor go

The companion-derived abilities map onto the search tools:

| Ability | In this design |
|---|---|
| **Light** | Core. Her glow is how you find her; the call's answer is a pulse of it. In chapter 5 the lantern from the temple is a small light you carry to search the dark alone. |
| **Boost** | Becomes **getting up high**. In chapter 2 she gives you a leg up onto a wall to see over the crowd (or you lift her); later you climb onto a crate. |
| **Anchor** | Becomes **pulling each other up** a ledge by the hand. Alone, you mantle slowly. This also answers the open "Anchor in 3D" question. |

---

## Vignettes: one per chapter, capped

Dean's concern is right: a new mechanic for every scene is the most expensive way to make a game like this. So the budget is strict.

### The rules

1. **One vignette per chapter**, at that chapter's key moment. Five in total, plus the Two Doors, which already exists.
2. **No new inputs.** A vignette may only use what the game already has: walk, call, look, hold hands, and one "hold to do" button.
3. **No new systems.** It may use one new scripted beat and at most two or three new animation clips.
4. **Designed once, played twice.** Every vignette has an echo later in the game (or in a blue door run) that reuses its space and animations with something missing.
5. **At most about three days each** on today's engine. If it grows past that, cut it back to a plain scene.

### The five

| Chapter | Moment | Vignette | Built from | Echo | Cost |
|---|---|---|---|---|---|
| **1** | Meeting her | You call into the dark and, for the first time, something answers. Find the answer. | Call, search | Chapter 5: call into the same dark; nothing answers | S (it's the core loop) |
| **2** | A meal at the stall | Sit beside her and eat: hold the button, bite by bite. She steals the last piece. | Hold to do (the food bites in Lantern's word market) | Chapter 5: the same stall, one bowl | S, mostly built |
| **2** | The Rose Petal Light Show (the peak) | Take a photo of the two of you, framed with her in the light. | Photo mode (built) | Chapter 5: the photo is the only thing in your hands. Blue door: you try to frame the same shot and the ghost won't stay in it | S, mostly built |
| **3** | The Hakone cable car | Sit beside her. Hold the button to hold her hand. Halfway up, her hand slips away, however hard you hold. | Hold hands, hold to do | Chapter 4: the train seat; she sits across from you, not beside you | S to M (two seated clips) |
| **4** | The Rose Garden | The thorns close between you. You see her glow through them and call. No answer, for the first time in the game. | Call, look | The Two Doors: you call again, and the ghost answers | S (scripted thorn beat exists) |
| **5** | Fade to Darkness | Keep walking with the lantern while the world goes dark. The call button still works. | Walk, call | The third blue door run | S (already designed) |

Chapter 2 has two because it is the peak, and both are already mostly built in Lantern. If the budget needs cutting, the meal goes first.

---

## First or third person

**Third person.** Seeing the two of them together is the core image of the game, and the distance between their silhouettes is what the camera arc is built on. Third person also fits the authored shots, the painterly look and the RDR2-style sense of a body with weight, and it costs less than first person, which needs good-looking arms and hands on screen for every interaction.

Borrow first person's intimacy with the camera instead:

| When | Framing |
|---|---|
| **Together** | Authored shots or a relaxed over-the-shoulder follow, with both of them in frame. Close in chapters 1 and 2, pulling back through 3 and 4. |
| **Searching** | Tight over the shoulder, low, a slightly longer lens, crowd figures crossing the frame and blocking the view. You feel inside the crowd, close to first person, but you still see him. |
| **Look** | Eases in further, almost over his eyes, for as long as you hold it. |
| **After she's gone** | Wide and still. Him small in an empty frame. The places you searched now shown from far away, with nobody in them. |

---

## How to get weight without realism

Red Dead Redemption 2's characters feel heavy because of **how they move through time**, not because of the detail of their models. Almost all of that is timing, and timing is cheap. The faceless silhouettes help a great deal: no faces, no lip sync, no cloth detail, so the effort goes into *how he moves*.

### Do (cheap, and most of the feeling)

| Technique | What it does | Lantern today |
|---|---|---|
| **Momentum** | Speed builds and fades; you can't stop dead or turn on the spot. | Built (grounded feel, ramp times) |
| **Turning arcs** | A wide turn at speed, a pivot when standing. Head and shoulders lead. | Partly; add a turn rate that falls with speed |
| **Head first** | He looks before he turns, and looks at her. This is also the Look tool. | Look-at layer exists |
| **Walk by default** | No sprint. A jog only while she's lost. | Grounded walk speed 4.4 m/s; slow it to taste |
| **A mood setting per chapter** | One number that drives stride, walk speed, posture (an added slump in the spine and head) and idle choice. Chapter 2 upright and light, chapter 5 slow, shoulders down. | New: an additive pose layer, S to M |
| **Few, long, authored moments** | Doors, sitting, eating, reaching for her hand. Slow canned clips rather than many short ones. | Animator, blend states and transitions exist |
| **Camera lag and lens** | A camera that trails, a longer lens, low heights, slow blends, a hint of handheld drift. | Follow rig, volumes and shots exist; add drift |
| **Sound** | Footsteps per surface, breath after a climb, clothing, the city. Half of the weight. | Footstep system exists; Cinder planned |
| **Pacing** | Long walks between moments, silence, waiting. | Level design |

### Don't (expensive, and little of the feeling)

- Motion matching or motion capture libraries
- Full-body IK (simple foot planting on stairs is optional polish, later)
- Ragdolls or physics-driven bodies
- Facial animation, speech or lip sync (ruled out by the faceless design anyway)
- Crowd AI with goals and schedules; crowd figures follow paths with a simple avoidance rule
- An open world; levels stay authored and mostly linear

### Crowds on a budget

Crowd figures are faceless shadow figures too, rougher than the two of them, which fits the story: everyone else is a blur. Each is a moving body on a path with a simple avoid rule (the "crowd flow" idea in [`grounded-adventure-ideas.md`](grounded-adventure-ideas.md)). One shared walk cycle with random phase and speed, instanced draws, simple shapes in the distance. Her glow and his warmth make the two of them the only figures with colour, so she is always findable by eye. A crowd figure turning its head towards where she went is the look-at layer, and doubles as a trace.

---

## Scope

Costs use Lantern's scale: **S** a day or less of Nyx, **M** a few days, **L** needs engine work.

### Core systems (built once, used everywhere)

| Piece | Lantern today | Cost |
|---|---|---|
| Grounded walk, hop, mantle, jog while she's lost | Built; jog is one more speed | S |
| Call and answer: input, positioned answer sound, glow pulse through walls | New | S to M |
| Look: hold to focus, camera ease, mix duck, trace highlight | Look-at layer and camera exist | S |
| Vantage spots: authored places to step up, with a wider shot | Mantle and camera volumes exist | S |
| Her hiding, wandering and moving on during a search | Team beats and the navigation graph exist; hiding spots are authored beats | M |
| Traces: petals, glow on walls, heads turning | Particles, lights and look-at exist | S to M |
| Hand-holding: a paired walk at her pace | Her follow exists; this is a fixed-offset mode | M |
| Crowds: path-following figures with avoidance, instanced | Designed, not built | M |
| Mood setting per chapter (posture, stride, speed) | New additive pose layer | S to M |
| 3D sound for her answer and the mix ducks | Cinder (planned); a placeholder positioned sound works first | Depends on Cinder |

About **three to five weeks** of Nyx for the whole core, with no engine work except Cinder, which is planned anyway.

### Content

- **Searches:** roughly two to four per level, built from the place-and-situation matrix above. Each is an authored set of hiding spots, traces and vantage points in an existing level, not a new mechanic.
- **Vignettes:** five, each S or S to M, most of them partly built already.
- **Levels:** the two recurring levels, the four unique levels and the train.

---

## What to prototype first

Prototype a moment, not a mechanic. Build **one festival crowd space in two chapter states**, in a single world file:

1. **Chapter 2:** walk in holding hands; the crowd pulls you apart; call, and she answers close; get up on a crate to see over the crowd; find her by the stall she liked a minute ago.
2. **Chapter 5:** the same space and the same crowd. You call; nothing answers. You go to the stall. The level ends when you stop.

Needs: crowd figures (M), call and answer (S to M), look (S), one vantage spot (S), hand-holding (M), the mood setting (S to M). About **one to two weeks**, no engine work. Play chapter 2, then chapter 5. If the second one lands, the game has its spine. If searching in chapter 2 already feels like wandering, add more partial information (stronger traces, shorter answer gaps) before adding anything new.

---

## Open questions for Dean

The prototype picks the doc's leaning for each and makes it one constant to swap; see [`finding-her-prototypes.md`](finding-her-prototypes.md).

- Should the call be his voice (a short sound), a whistle, or something wordless and abstract?
- In chapter 5, should the level end when you stop calling, or when you reach the exit? The first is stronger and riskier.
- How literal are the crowds: Tokyo commuters and festival-goers, or abstract shadow figures that flow like water?
- Are the chapter 2 meal and photo both worth keeping, or should chapter 2 have one vignette like the rest?

---

## Earlier directions (for context)

The first version of this page offered three directions inside a 3D platformer. This design is built from the third, with the first's hand-holding folded in.

- **Walk With Her:** holding her hand as the main verb; crowds make you let go. Kept here as the hand-holding mode and the cable car vignette.
- **The City Together:** the grounded, combat-free city adventure Lantern was heading towards after the 2026-11-16 play-test (climbing, the station, trains, stalls). Not chosen as the spine because levers and gates drift back towards puzzles and it needs the most content. Its pieces (stalls, photo mode, the station, mantling) remain useful as places and vignettes.
- **Finding Her:** every level a search, with a call whose answer fades over the chapters. This is now the whole design.
