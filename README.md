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
Types are essentially just parameters attached to certain operations to define the (de)serialization format.
Well-formedness of a MiniRust program ensures that expressions and statements satisfy some basic typing discipline, but MiniRust is by design *not* type-safe.

## How to read MiniRust

The most precise way to write down the MiniRust spec would be with mathematical notation.
However, without LaTeX this is a pain, and it also involves a lot of jargon which hurts accessibility of the spec.
Therefore, the spec is written in a kind of "pseudo Rust" (or "OCaml with Rust syntax"):
imagine Rust without all the restrictions about sizendess and pointer indirections for recursive types (we could implicitly insert `Arc` where needed).
We use generic type names like `List`, `Map`, `Set` rather than concrete implementations like `Vec`, `HashMap`, `HashSet`, since the implementation details do not matter.
Also, all types except for mutable references are `Copy` (let's just imagine we implicitly `Clone` where needed), and we use `fn(T) -> U` notation even for closures that can capture arbitrarily.
We also assume some "obvious" language extensions -- basically, it should always be clear what is meant to anyone with some Rust experience, even if this is not actually legal Rust.

We use `Result` to make operations fallible (where failure indicates UB or machine termination), and omit trailing `Ok(())` and `Some(())`.
We use a `throw_ub!` macro to make the current function return a UB error, and `throw_machine_step!` to indicate that and how the machine has stopped.
We use `panic!` (and `unwrap` and similar standard Rust operations) to indicate conditions that should always hold; if execution ever panics, that is a bug in the specification.

We also need one language feature that Rust does not have direct support for: functions returning `Result` can exhibit non-determinism.
(If you are a monad kind of person, think of `Result` as also containing the non-determinism monad, not just the error monad.)
This is only used in the memory model; the expression language is perfectly deterministic.
The function `pick<T>(fn(T) -> bool) -> Result<T>` will return a value of type `T` such that the given closure returns `true` for this value.
This non-determinism is interpreted *daemonically*, which means that the program has to be correct for every possible choice.
In particular, if the closure is `|_| false` or `T` is uninhabited, then this corresponds to "no behavior" (which is basically the perfect opposite of Undefined Behavior, and also very confusing).
Similarly, the function `predict<T>(fn(T) -> bool) -> Result<T>` also returns a `T` satisfying the closure, but this non-determinism is interpreted *angelically*, which means there has to *exist* a possible choice that makes the program behave as intended.
In particular, if the closure is `|_| false` or `T` is uninhabited, then this operation is exactly the same as `hint::unreachable_unchecked()`: no possible choice exists, and hence ever reaching this operation is Undefined Behavior.

## Status

MiniRust is extremely incomplete!
Many features are entirely missing (e.g. floats, unsized types) or only partially sketched (enum layouts).
Many types have missing representation relations.
There are lots of TODOs.
The language syntax is also missing many of the Rust operators and casts.
I hope to slowly chip away at all this over time.
If you want to help, please talk to me -- PRs to add missing features are very welcome. :)
But we also need to ensure the entire document stays coherent, and I already have vague ideas for many of these things.

- TODO: establish global variable name conventions. Do we use `v: Value`, `val: Value`, `value: Value`?
  What do we use for `ValueExpr`? Similar questions exist around `Place`/`PlaceExpr` and `ty: Type`/`type: Type`.

## What about a-mir-formality?

You might wonder how this project compares to Niko's [a-mir-formality](https://github.com/nikomatsakis/a-mir-formality/).
The obvious answer is that Niko is much better at picking names. ;)

On a more serious note, these projects have very different scope: MiniRust is *only* about the operational semantics.
a-mir-formality is a lot more ambitious; as the [inaugurate blog post](https://nikomatsakis.github.io/a-mir-formality/blog/2022/05/12/) explains, it aims to also formalize traits, type checking, and borrow checking -- all of which I consider out-of-scope for MiniRust.
a-mir-formality is machine-readable and written in PLT redex; MiniRust uses pseudo-code that is not currently machine-readable (but I have ideas :).
The primary goals of MiniRust are to be precise and human-readable; I would argue that while PLT redex is more precise than the style I use, it does lack in readability when compared with Rust-style pseudo-code.
I am willing to sacrifice some precision for the significant gain in readability, in particular since I think we can recover this precision with some amount of work.
And finally, the "operational semantics" layer in a-mir-formality is "not even sketched out yet", so as of now, the projects are actually disjoint.
If and when a-mir-formality obtains an operational semantics, my hope is that it will be basically the same as MiniRust, just translated into PLT redex.
(Niko writes this layer of a-mir-formality is "basically equivalent to Miri"; MiniRust is basically an idealized Miri, so I think this would work well.)

## What about Miri?

MiniRust is in, at least conceptually, very similar to [Miri](https://github.com/rust-lang/miri).
You can think of it as "idealized Miri": if Miri didn't have to care about working with all the rustc data structures that represent MIR and types, and didn't care about performance nor diagnostics, then it would be implemented like this specification.
There are some [differences between Miri and MiniRust](https://github.com/rust-lang/miri/issues/2159); these are generally Miri bugs and I intend to slowly chip away at the remaining (tricky) ones.

## Table of Contents

* [Prelude](prelude.md): common definitions and parameters shared by everything
* MiniRust memory
  * [Memory interface](mem/interface.md): the API via which the MiniRust Abstract Machine interacts with memory
* MiniRust language
  * [Prelude](lang/prelude.md): common definitions and parameters of the language
  * [Values and Types](lang/values.md): the domain of high-level MiniRust values and how types can be used to (de)serialize them to memory
  * [Syntax](lang/syntax.md): the abstract syntax of MiniRust programs
  * [Well-formedness](lang/well-formed.md): the requirements for well-formed types and programs
  * [Abstract Machine](lang/machine.md): the state that makes up a MiniRust Abstract Machine
  * [Semantics](lang/step.md): the operational semantics ("`step` function") of the Abstract Machine
    * [Operator semantics](lang/operator.md): the operational semantics of unary and binary operators
