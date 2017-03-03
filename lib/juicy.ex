defmodule Juicy do
  @moduledoc """
  Documentation for Juicy.
  """

  def parse(binary) do
    Juicy.Basic.parse(binary)
  end

  def parse_stream(stream, spec) do
    Juicy.Stream.stream(stream, spec)
  end

  def validate_spec(spec) do
    Juicy.Native.validate_spec(spec)
  end

end
