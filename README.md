# MiniRust

MiniRust is the cornerstone of my vision for a normative specification of Rust semantics.
It is an idealized MIR-like language with the purpose of serving as a "core language" of Rust.
This is part of a grater story whose goal is to precisely specify the operational behavior of Rust, i.e., the possible behaviors that a Rust program might have when being executed:
the behavior of a Rust program is defined by first translating it to MiniRust (which is outside the scope of this repository), and then considering the possible behaviors of the MiniRust program as specified in this document.

To separate the complexities of memory from the semantics of MiniRust statements and expressions, we introduce the MiniRust *memory interface*:
think of memory as implementing some trait; MiniRust semantics is generic over the actual implementation of that trait.
The interface between the MiniRust language (specified in `lang`) and its memory model (specified in `mem`) is *untyped and byte-oriented* (but "bytes" are a bit more complex than you might expect).
For now, we only define the memory interface, but do not give an implementation.
Even without deciding what exactly the final memory model will look like, we can answer a surprising amount of interesting questions about Rust semantics!

On the MiniRust langauge side, the most important concept to understand is that of a *value* and how it relates to *types*.
Values form a high-level, structural view of data (e.g. mathematical integers); types serve to relate values and their low-level byte-oriented representation.
Types are just parameters attached to certain operations to define the (de)serialization format.
There is no MiniRust type system (as in, typing rules that would define when a MiniRust program is "well-typed").
(We might have a type system in the future as a basic sanity check, but MiniRust is by design *not* type-safe.)

## How to read MiniRust

The most precise way to write down the MiniRust spec would be with mathematical notation.
However, without LaTeX this is a pain, and it also involves a lot of jargon which hurts accessibility of the spec.
Therefore, the spec is written in a kind of "pseudo Rust" (or "OCaml with Rust syntax"):
imagine Rust without all the restrictions about sizendess and pointer indirections for recursive types.
Also, all types are `Copy` (let's just imagine we implicitly `Clone` where needed), and we use `fn(T) -> U` notation even for closures that can capture arbitrarily.
We also assume some "obvious" language extensions -- basically, it should always be clear what is meant to anyone with some Rust experience, even if this is not actually legal Rust.

We also need one language feature that Rust does not have direct support for: non-determinism.
The function `pick<T>(fn(T) -> bool) -> T` will return a value of type `T` such that the given closure returns `true` for this value.
(If there is no such value, the function does not return. This is a bug, the spec should never do that.
This non-determinism is interpreted *daemonically*, which means that the compiler can refine it arbitrarily and the program has to be correct for every possible choice.)

## Status

MiniRust is extremely incomplete!
Many features are entirely missing (e.g. floats, unsized types) or only partially sketched (enum layouts).
Many types have missing representation relations.
The language syntax is also missing many of the Rust operators and casts.
If you want to help, please talk to me -- PRs to add missing features are very welcome. :)

## Table of Contents

* [Prelude](prelude.md)
* MiniRust memory
  * [Memory interface](mem/interface.md): the API via which the MiniRust AM interacts with memory
* MiniRust language
  * [Prelude](lang/prelude.md)
  * [Values](lang/values.md): the domain of high-level MiniRust values
  * [Types](lang/types.md): the set of MiniRust types **and how they relate values with their representation** (a key part of the language)
  * [Syntax](lang/syntax.md): the syntax of MiniRust programs
  * [Abstract Machine](lang/am.md): the state that makes up a MiniRust Abstract Machine (AM)
  * [Semantics](lang/sem.md): the operational semantics of the MiniRust Abstract Machine
