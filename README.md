# MiniRust

MiniRust is the cornerstone of my vision for a normative specification of Rust semantics.
It is an idealized MIR-like language with the purpose of serving as a "core language" of Rust.
This is part of a larger story whose goal is to precisely specify the operational behavior of Rust, i.e., the possible behaviors that a Rust program might have when being executed:
the behavior of a Rust program is defined by first translating it to MiniRust (which is outside the scope of this repository), and then considering the possible behaviors of the MiniRust program as specified in this document.
That translation does a *lot* of work; for example, traits and pattern matching are basically gone on the level of MiniRust.
On the other hand, MiniRust is concerned a lot with details such as the exact evaluation order, data representations, and precisely what is and is not Undefined Behavior.

To separate the complexities of memory from the semantics of MiniRust statements and expressions, we introduce the MiniRust *memory interface*:
think of memory as implementing some trait; MiniRust semantics is generic over the actual implementation of that trait.
The interface between the MiniRust language (specified in `lang`) and its memory model (specified in `mem`) is *untyped and byte-oriented* (but "bytes" are a bit more complex than you might expect).
For now, we only define the memory interface, but do not give an implementation.
Even without deciding what exactly the final memory model will look like, we can answer a surprising amount of interesting questions about Rust semantics!

On the MiniRust language side, the most important concept to understand is that of a *value* and how it relates to *types*.
Values form a high-level, structural view of data (e.g. mathematical integers); types serve to relate values with their low-level byte-oriented representation.
Types are just parameters attached to certain operations to define the (de)serialization format.
There is no MiniRust type system (as in, typing rules that would define when a MiniRust program is "well-typed").
We might have a type system in the future as a basic sanity check, but MiniRust is by design *not* type-safe.

## How to read MiniRust

The most precise way to write down the MiniRust spec would be with mathematical notation.
However, without LaTeX this is a pain, and it also involves a lot of jargon which hurts accessibility of the spec.
Therefore, the spec is written in a kind of "pseudo Rust" (or "OCaml with Rust syntax"):
imagine Rust without all the restrictions about sizendess and pointer indirections for recursive types.
We use generic type names like `List`, `Map`, `Set` rather than concrete implementations like `Vec`, `HashMap`, `HashSet`, since the implementation details do not matter.
Also, all types are `Copy` (let's just imagine we implicitly `Clone` where needed), and we use `fn(T) -> U` notation even for closures that can capture arbitrarily.
We also assume some "obvious" language extensions -- basically, it should always be clear what is meant to anyone with some Rust experience, even if this is not actually legal Rust.

We use `Result` to make operations fallible (where failure indicates UB), and omit trailing `Ok(())`.
We use a `throw_ub!` macro to make the current function return a UB error.
We use `panic!` (and `unwrap` and similar standard Rust operations) to indicate conditions that should always hold; if execution ever panics, that is a bug in the specification.

We also need one language feature that Rust does not have direct support for: non-determinism.
The function `pick<T>(fn(T) -> bool) -> T` will return a value of type `T` such that the given closure returns `true` for this value.
If there is no such value, the function does not return. This is a bug, the spec should never do that.
This non-determinism is interpreted *daemonically*, which means that the compiler can refine it arbitrarily and the program has to be correct for every possible choice.

## Status

MiniRust is extremely incomplete!
Many features are entirely missing (e.g. floats, unsized types) or only partially sketched (enum layouts).
Many types have missing representation relations.
There are lots of TODOs.
The language syntax is also missing many of the Rust operators and casts.
I hope to slowly chip away at all this over time.
If you want to help, please talk to me -- PRs to add missing features are very welcome. :)
But we also need to ensure the entire document stays coherent, and I already have vague ideas for many of these things.

## Table of Contents

* [Prelude](prelude.md): common definitions and parameters shared by everything
* MiniRust memory
  * [Memory interface](mem/interface.md): the API via which the MiniRust Abstract Machine interacts with memory
* MiniRust language
  * [Prelude](lang/prelude.md): common definitions and parameters of the language
  * [Values and Types](lang/values.md): the domain of high-level MiniRust values and how types can be used to (de)serialize them to memory
  * [Syntax](lang/syntax.md): the abstract syntax of MiniRust programs
  * [Abstract Machine](lang/machine.md): the state that makes up a MiniRust Abstract Machine (AM)
  * [Semantics](lang/step.md): the operational semantics ("`step` function") of the MiniRust Abstract Machine
