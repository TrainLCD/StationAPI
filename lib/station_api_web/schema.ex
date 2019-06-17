defmodule StationApiWeb.Schema do
  use Absinthe.Schema
  import_types StationApiWeb.Schema.ContentTypes

  alias StationApiWeb.Resolvers

  query do
    @desc "Fetch station by coorinates"
    field :station_by_coords, :station do
      arg :latitude, non_null(:float)
      arg :longitude, non_null(:float)
      resolve &Resolvers.Station.station_by_coords/3
    end
  end
end
