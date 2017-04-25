defmodule Juicy.Native do
  @moduledoc false

  use Rustler, otp_app: :juicy, crate: "juicy_native"

  def parse_init(_), do: err()
  def parse_iter(_, _, _), do: err()

  def spec_parse_init(_, _), do: err()
  def spec_parse_iter(_), do: err()

  def stream_parse_init(_), do: err()
  def stream_parse_iter(_, _), do: err()

  def validate_spec(_), do: err()

  defp err, do: throw NifNotLoadedError
end
