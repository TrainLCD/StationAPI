defmodule StationApiWeb.Schema do
  use Absinthe.Schema
  import_types StationApiWeb.Schema.ContentTypes

  alias StationApiWeb.Resolvers

  query do
    @desc "Fetch station by coordinates"
    field :station_by_coords, :station do
      arg :latitude, non_null(:float)
      arg :longitude, non_null(:float)
      resolve &Resolvers.Station.station_by_coords/3
    end
    @desc "Fetch station by ID"
    field :station, :station do
      arg :id, non_null(:id)
      resolve &Resolvers.Station.station_by_id/3
    end
    @desc "Fetch line by ID"
    field :line, :line do
      arg :id, non_null(:id)
      resolve &Resolvers.Line.line_by_id/3
    end
    @desc "Fetch stations by Line ID"
    field :stations_by_line_id, list_of(:station) do
      arg :line_id, non_null(:id)
      resolve &Resolvers.Station.stations_by_line_id/3
    end
  end
end
