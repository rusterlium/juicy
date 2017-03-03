# Juicy

Juicy is a JSON parser written as a NIF in the Rust language.

**Juicy is currently in active development. It is not production ready yet.**

Features:

* Safety - Juicy is written entirely in safe Rust code. This means that there is a lot less ways it can crash the VM then something written in C. I have not had a single VM crash while developing this.
* Speed - Juicy is comparible in speed to jiffy, and beats Poison at all benchmarks I have tried so far.
* Streaming - Juicy supports parsing a stream of JSON. This makes it possible to parse very large JSON files while avoiding keeping the whole file in memory.
* Convenience - Juicy supports parsing JSON directly into a rigidly defined schema. That includes maps with atom keys and elixir structs. **Not implemented**
* UTF-8 compliance - Juicy is fully UTF-8 compliant. All invalid unicode codepoints result in parse errors.

It also has some disadvantages:

* NIF - Being a NIF written in Rust, you need the Rust compiler installed to compile it. Using native code also complicates cross-compilation. There is also a higher risk of something bad happening to the VM when using a NIF.
* No encoder - The project is currently focused on doing JSON parsing right, and does not have an encoder. This will probably change in the future.

## Installation

If [available in Hex](https://hex.pm/docs/publish), the package can be installed
by adding `juicy` to your list of dependencies in `mix.exs`:

```elixir
def deps do
  [{:juicy, "~> 0.1.0"}]
end
```

Documentation can be generated with [ExDoc](https://github.com/elixir-lang/ex_doc)
and published on [HexDocs](https://hexdocs.pm). Once published, the docs can
be found at [https://hexdocs.pm/jinx](https://hexdocs.pm/jinx).

