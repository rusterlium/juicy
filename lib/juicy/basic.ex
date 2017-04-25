defmodule Juicy.Basic do
  @moduledoc false

  def parse(binary) do
    handle_parse_return(binary, Juicy.Native.parse_init(binary))
  end

  defp handle_parse_return(binary, {:iter, stack, res}) do
    handle_parse_return(binary, Juicy.Native.parse_iter(binary, stack, res))
  end
  defp handle_parse_return(_, ret), do: ret

  def parse_spec(binary, spec) do
    {:ok, state} = Juicy.Native.spec_parse_init(binary, spec)
    handle_parse_spec_return(Juicy.Native.spec_parse_iter(state))
  end

  defp handle_parse_spec_return({:iter, state}) do
    handle_parse_spec_return(Juicy.Native.spec_parse_iter(state))
  end
  defp handle_parse_spec_return(resp), do: resp

end
