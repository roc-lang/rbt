# Principles

These principles inform how we make decisions about rbt, both in how it should feel to use and as standards for internals:

1. **Be Transparent**

   As an rbt user, you should be able to observe what rbt is doing at any given time: it should never block or hang without any kind of feedback.
   You should be able to understand what rbt will do once you've written down your build instructions.

   We work to minimize hidden or undocumented variables that can come into play to break your build, and try to make sure that rbt supports you in these situations.

2. **Be Approachable**

   When we say that rbt should be approachable, we're thinking about teams producing software.
   Say the build needs some adjustments: do you need to be an expert in the build system, or can you just go look at the code, make intuitive changes, try them out, and get a good result?
   Approachability makes it easier for people to learn what their build steps are doing, and avoids creating pressure or silos on individuals or small groups of developers on a team.

   When it comes to internal code, approachability means we try to keep internals simple and well-documented.
   We don't shy away from complexity where it's warranted, but we think that straightforward and readable code is valuable and will help keep the system working well.
   (After all, "a complex system that works is invariably found to have evolved from a simple system that worked.")

3. **Be Fast**

   This has one plain meaning: rbt should run a build to completion as quickly as possible.

   However, since build systems are typically bound by the speed of the tools you're using to build, rbt also focuses on perceived speed: we show you feedback as soon as possible and work hard to minimize startup costs.

---

These are listed in order of importance.
That is to say: given the choice between transparency or approachability, we'll choose transparency (e.g. by documenting unavoidable complexity.)
Given the choice between approachability and speed, we'll choose approachability.
