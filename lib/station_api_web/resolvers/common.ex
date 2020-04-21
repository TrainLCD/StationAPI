defmodule StationApiWeb.Resolvers.Common do
  def to_atomic_map(str_map) do
    str_map |> Map.new(fn {k, v} -> {String.to_atom(k), v} end)
  end

  def transform_line_result(map) do
    %{
      id: map[:line_cd],
      company_id: map[:company_cd],
      latitude: Decimal.to_float(map[:lat]),
      line_color_c: map[:line_color_c],
      line_color_t: map[:line_color_t],
      name: map[:line_name],
      name_h: map[:line_name_h],
      name_k: map[:line_name_k],
      name_r: map[:line_name_r],
      line_type: map[:line_type],
      longitude: Decimal.to_float(map[:lon]),
      zoom: map[:zoom]
    }
  end
end
