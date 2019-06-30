defmodule StationApiWeb.Resolvers.Line do
  import StationApiWeb.Resolvers.Common

  def line_by_id(_parent, %{id: id}, _resolution) do
    {:ok, lines} = StationApi.Line.line_by_id(id)
    line = List.first(lines)

    case line do
      nil -> {:error, "Line not found."}
      line ->
        line_map = to_atomic_map(line)
        api_result = transform_line_result(line_map)

        {:ok, api_result}
    end
  end
end
