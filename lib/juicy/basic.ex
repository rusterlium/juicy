defmodule Juicy.Basic do

  def parse(binary) do
    handle_parse_return(binary, Juicy.Native.parse_init(binary))
  end

  defp handle_parse_return(binary, {:iter, stack, res}) do
    handle_parse_return(binary, Juicy.Native.parse_iter(binary, stack, res))
  end
  defp handle_parse_return(_, ret) do
    ret
  end

end
