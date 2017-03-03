defmodule JuicyTest do
  use ExUnit.Case
  doctest Juicy

  def p(binary), do: Juicy.parse(binary)

  test "empty objects" do
    assert p("{}") == {:ok, %{}}
    assert p("[]") == {:ok, []}
  end

  test "basic strings" do
    assert p(~s(["woo", "hoo"])) == {:ok, ["woo", "hoo"]}
  end

  test "character escapes" do
    input = ~s(["\\"", "\\\\", "\/", "\\b", "\\f", "\\n", "\\r", "\\t"])
    output = {:ok, ["\"", "\\", "/", "\b", "\f", "\n", "\r", "\t"]}
    assert p(input) == output
  end

  test "unicode escapes" do
    input = ~s(["\\u00E5"])
    output = {:ok, ["\u00E5"]}
    assert p(input) == output
  end

  test "integers" do
    assert p("[1, -1, 9999]") == {:ok, [1, -1, 9999]}
  end

  test "floats" do
    assert p("[1.0, 1.0e0, -1e-1]") == {:ok, [1.0, 1.0, -0.1]}
  end

  test "large integers" do
    input = ~s([9999999999999999999999999999999999999999])
    output = {:ok, [9999999999999999999999999999999999999999]}
    assert p(input) == output
  end

  test "match spec validation" do
    assert :ok == Juicy.validate_spec({:map, [], {:any, []}})
    assert :ok == Juicy.validate_spec({:map, [], {:any, [stream: true]}})
    assert :ok == Juicy.validate_spec({:map_keys, [], %{"a" => {:any, []}}})
    assert :ok == Juicy.validate_spec({:map, [atom_mappings: %{"some" => :some}], {:any, []}})
    assert :error == Juicy.validate_spec(nil)
    assert :error == Juicy.validate_spec({:abc, [], {:any, []}})
    assert :error == Juicy.validate_spec({:map_keys, [], %{0 => {:any, []}}})
  end

  test "basic stream" do
    input = ["{\"w", "oo\":", " [12, 2", "3, 34]}"]
    spec = {:map, [stream: true], {:array, [], {:any, [stream: true]}}}
    out = Juicy.parse_stream(input, spec) |> Enum.into([])

    assert out == [
      {:yield, {["woo", 0], 12}},
      {:yield, {["woo", 1], 23}},
      {:yield, {["woo", 2], 34}},
      {:yield, {[], %{"woo" => [:streamed, :streamed, :streamed]}}},
      :finished,
    ]
  end

  test "early end of input stream" do
    input = ["{"]
    spec = {:any, []}
    out = Juicy.parse_stream(input, spec) |> Enum.into([])

    assert out == [error: :early_eoi]
  end

  test "json from input stream" do
    device = File.stream!("")
  end

end
