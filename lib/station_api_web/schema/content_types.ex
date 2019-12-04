defmodule StationApiWeb.Schema.ContentTypes do
  use Absinthe.Schema.Notation

  object :station do
    field :id, :id
    field :address, :string
    field :distance, :float
    field :latitude, :float
    field :longitude, :float
    field :lines, list_of(:line)
    field :open_ymd, :string
    field :postal_code, :string
    field :pref_id, :integer
    field :group_id, :integer
    field :name, :string
    field :name_k, :string
    field :name_r, :string
  end

  object :line do
    field :id, :id
    field :company_id, :integer
    field :latitutde, :float
    field :line_color_c, :string
    field :line_color_t, :string
    field :name, :string
    field :name_h, :string
    field :name_k, :string
    field :line_type, :string
    field :longitude, :float
    field :zoom, :integer
  end
end
