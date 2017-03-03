defmodule Juicy.Stream do

  defstruct reader: nil, spec: nil, binaries: nil, parser: nil, output_queue: [], state: {:read_input, :parsing_not_done, nil}

  def stream(input, spec) do
    reader = input
    |> Stream.transform(0, fn(elem, pos) -> {[{pos, elem}], pos+byte_size(elem)} end)
    |> stream_take_init

    %__MODULE__{
      reader: reader,
      spec: spec,
    }
  end

  def stream_take_init(stream) do
    reduce_fun = fn(elem, nil) -> {:suspend, elem} end
    {:suspended, nil, next_fun} = Enumerable.reduce(stream, {:suspend, nil}, reduce_fun)
    next_fun
  end
  def stream_take_next(next_fun) do
    next_fun.({:cont, nil})
  end
  def stream_take_halt(next_fun) do
    next_fun.({:halt, nil})
  end

end

defimpl Enumerable, for: Juicy.Stream do

  def count(_stream) do
    {:error, __MODULE__}
  end
  def member?(_stream) do
    {:error, __MODULE__}
  end

  def reduce(js = %Juicy.Stream{}, acc, fun) do
    {:ok, parser} = Juicy.Native.stream_parse_init(js.spec)
    js = %Juicy.Stream{ js |
            parser: parser,
            binaries: [],
          }
    do_reduce(js, acc, fun)
  end

  defp do_reduce(js, {:halt, acc}, fun) do
    Juicy.Stream.stream_take_halt(js.reader)
    {:halted, acc}
  end
  defp do_reduce(js, {:suspend, acc}, fun) do
    {:suspended, acc, &do_reduce(js, &1, fun)}
  end

  defp do_reduce(js, {:cont, acc}, fun) do
    {transition, js} =
      case js.state do

        {:read_input, :parsing_not_done, _} ->
          case Juicy.Stream.stream_take_next(js.reader) do
            {:suspended, binary, reader} ->
              js = %{js |
                     binaries: [binary | js.binaries],
                     reader: reader,
                     state: {:parse, :parsing_not_done, nil},
                    }
              {:loop, js}
            {:halted, _} ->
              js = %{js |
                     state: {:emit_items, :parsing_done, nil},
                     output_queue: [{:error, :early_eoi}],
                    }
              {:loop, js}
          end

        {:parse, :parsing_not_done, _} ->
          {status, yields, binaries, state} = Juicy.Native.stream_parse_iter(js.binaries, js.parser)
          js = %{js | output_queue: yields, parser: state, binaries: binaries}
          case status do
            :finished -> {:loop, %{js | state: {:emit_items, :parsing_done, nil}}}
            :iter -> {:loop, %{js | state: {:emit_items, :parsing_not_done, nil}}}
            :await_input -> {:loop, %{js | state: {:emit_items, :parsing_not_done, :await_input}}}
          end

        {:emit_items, :parsing_done, _} ->
          case js.output_queue do
            [] -> {:done, js}
            _ -> {:emit_output_item, js}
          end

        {:emit_items, :parsing_not_done, :await_input} ->
          case js.output_queue do
            [] -> {:loop, %{js | state: {:read_input, :parsing_not_done, nil}}}
            _ -> {:emit_output_item, js}
          end

        {:emit_items, :parsing_not_done, _} ->
          case js.output_queue do
            [] -> {:loop, %{js | state: {:parse, :parsing_not_done, nil}}}
            _ -> {:emit_output_item, js}
          end

      end

    case transition do
      :loop -> do_reduce(js, {:cont, acc}, fun)
      :emit_output_item ->
        [head | tail] = js.output_queue
        js = %{js | output_queue: tail}
        do_reduce(js, fun.(head, acc), fun)
      :done -> {:done, acc}
    end
  end

end
