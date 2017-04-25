defmodule Juicy do
  @moduledoc """
  Documentation for Juicy.
  """

  @type ejson :: any
  @type error :: any

  @doc """
  The simplest and most basic parse function.

  Will parse the input binary into ejson. Will return an error tuple if there are any
  errors in the input.

  This is the fastest parse function, as it does not need to walk a spec tree.
  """
  @spec parse(binary) :: {:ok, ejson} | {:error, error}
  def parse(binary) do
    Juicy.Basic.parse(binary)
  end

  @spec parse_spec(binary, Juicy.Spec.t) :: {:ok, any} | {:error, error}
  def parse_spec(binary, spec) do
    Juicy.Basic.parse_spec(binary, spec)
  end

  @spec parse_stream(Stream.t, Juicy.Spec.t) :: Stream.t
  def parse_stream(stream, spec) do
    Juicy.Stream.stream(stream, spec)
  end

  @spec validate_spec(Juicy.Spec.t) :: boolean
  def validate_spec(spec) do
    Juicy.Native.validate_spec(spec)
  end

end
