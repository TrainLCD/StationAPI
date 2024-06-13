CREATE SCHEMA stationapi;
ALTER SCHEMA stationapi OWNER TO stationapi;
--
-- Name: aliases; Type: TABLE; Schema: stationapi; Owner: stationapi
--

DROP TABLE IF EXISTS stationapi.aliases;
CREATE TABLE stationapi.aliases (
  id serial NOT NULL,
  line_name character varying(255),
  line_name_k character varying(255),
  line_name_h character varying(255),
  line_name_r character varying(255),
  line_name_zh character varying(255),
  line_name_ko character varying(255),
  line_color_c character varying(255)
);
ALTER TABLE stationapi.aliases OWNER TO stationapi;
--
-- Name: aliases_id_seq; Type: SEQUENCE; Schema: stationapi; Owner: stationapi
--

CREATE SEQUENCE stationapi.aliases_id_seq START WITH 1 INCREMENT BY 1 NO MINVALUE NO MAXVALUE CACHE 1;
ALTER SEQUENCE stationapi.aliases_id_seq OWNER TO stationapi;
--
-- Name: aliases_id_seq; Type: SEQUENCE OWNED BY; Schema: stationapi; Owner: stationapi
--

ALTER SEQUENCE stationapi.aliases_id_seq OWNED BY stationapi.aliases.id;
--
-- Name: companies; Type: TABLE; Schema: stationapi; Owner: stationapi
--

DROP TABLE IF EXISTS stationapi.companies;
CREATE TABLE stationapi.companies (
  company_cd integer NOT NULL,
  rr_cd integer NOT NULL,
  company_name character varying(255) NOT NULL,
  company_name_k character varying(255) NOT NULL,
  company_name_h character varying(255) NOT NULL,
  company_name_r character varying(255) NOT NULL,
  company_name_en character varying(255) NOT NULL,
  company_name_full_en character varying(255) NOT NULL,
  company_url character varying(255),
  company_type integer NOT NULL,
  e_status integer NOT NULL,
  e_sort integer NOT NULL
);
ALTER TABLE stationapi.companies OWNER TO stationapi;
--
-- Name: line_aliases; Type: TABLE; Schema: stationapi; Owner: stationapi
--

DROP TABLE IF EXISTS stationapi.line_aliases;
CREATE TABLE stationapi.line_aliases (
  id serial NOT NULL,
  station_cd integer NOT NULL,
  alias_cd integer NOT NULL
);
ALTER TABLE stationapi.line_aliases OWNER TO stationapi;
--
-- Name: line_aliases_id_seq; Type: SEQUENCE; Schema: stationapi; Owner: stationapi
--

CREATE SEQUENCE stationapi.line_aliases_id_seq START WITH 1 INCREMENT BY 1 NO MINVALUE NO MAXVALUE CACHE 1;
ALTER SEQUENCE stationapi.line_aliases_id_seq OWNER TO stationapi;
--
-- Name: line_aliases_id_seq; Type: SEQUENCE OWNED BY; Schema: stationapi; Owner: stationapi
--

ALTER SEQUENCE stationapi.line_aliases_id_seq OWNED BY stationapi.line_aliases.id;
--
-- Name: lines; Type: TABLE; Schema: stationapi; Owner: stationapi
--

DROP TABLE IF EXISTS stationapi.lines;
CREATE TABLE stationapi.lines (
  line_cd integer NOT NULL,
  company_cd integer NOT NULL,
  line_name character varying(255) NOT NULL,
  line_name_k character varying(255) NOT NULL,
  line_name_h character varying(255) NOT NULL,
  line_name_r character varying(255) NOT NULL,
  line_name_zh character varying(255),
  line_name_ko character varying(255),
  line_color_c character varying(255) NOT NULL,
  line_type integer NOT NULL,
  line_symbol_primary character varying(255),
  line_symbol_secondary character varying(255),
  line_symbol_extra character varying(255),
  line_symbol_primary_color character varying(255),
  line_symbol_secondary_color character varying(255),
  line_symbol_extra_color character varying(255),
  line_symbol_primary_shape character varying(255),
  line_symbol_secondary_shape character varying(255),
  line_symbol_extra_shape character varying(255),
  e_status integer NOT NULL,
  e_sort integer NOT NULL,
  average_distance double precision NOT NULL
);
ALTER TABLE stationapi.lines OWNER TO stationapi;
--
-- Name: station_station_types; Type: TABLE; Schema: stationapi; Owner: stationapi
--

DROP TABLE IF EXISTS stationapi.station_station_types;
CREATE TABLE stationapi.station_station_types (
  id serial NOT NULL,
  station_cd integer NOT NULL,
  type_cd integer NOT NULL,
  line_group_cd integer NOT NULL,
  pass integer DEFAULT 0 NOT NULL
);
ALTER TABLE stationapi.station_station_types OWNER TO stationapi;
--
-- Name: station_station_types_id_seq; Type: SEQUENCE; Schema: stationapi; Owner: stationapi
--

CREATE SEQUENCE stationapi.station_station_types_id_seq START WITH 1 INCREMENT BY 1 NO MINVALUE NO MAXVALUE CACHE 1;
ALTER SEQUENCE stationapi.station_station_types_id_seq OWNER TO stationapi;
--
-- Name: station_station_types_id_seq; Type: SEQUENCE OWNED BY; Schema: stationapi; Owner: stationapi
--

ALTER SEQUENCE stationapi.station_station_types_id_seq OWNED BY stationapi.station_station_types.id;
--
-- Name: stations; Type: TABLE; Schema: stationapi; Owner: stationapi
--

DROP TABLE IF EXISTS stationapi.stations;
CREATE TABLE stationapi.stations (
  station_cd integer NOT NULL,
  station_g_cd integer NOT NULL,
  station_name character varying(255) NOT NULL,
  station_name_k character varying(255) NOT NULL,
  station_name_r character varying(255),
  station_name_zh character varying(255),
  station_name_ko character varying(255),
  primary_station_number character varying(255),
  secondary_station_number character varying(255),
  extra_station_number character varying(255),
  three_letter_code character varying(255),
  line_cd integer NOT NULL,
  pref_cd integer NOT NULL,
  post character varying(255) NOT NULL,
  address character varying(255) NOT NULL,
  lon double precision NOT NULL,
  lat double precision NOT NULL,
  open_ymd character varying(255) NOT NULL,
  close_ymd character varying(255) NOT NULL,
  e_status integer NOT NULL,
  e_sort integer NOT NULL
);
ALTER TABLE stationapi.stations OWNER TO stationapi;
-- CREATE INDEX stations_name_r_ignore_accents_idx ON stationapi.stations(station_name_r COLLATE ignore_accents);
--
-- Name: types; Type: TABLE; Schema: stationapi; Owner: stationapi
--

DROP TABLE IF EXISTS stationapi.types;
CREATE TABLE stationapi.types (
  id serial NOT NULL,
  type_cd integer NOT NULL,
  type_name character varying(255) NOT NULL,
  type_name_k character varying(255) NOT NULL,
  type_name_r character varying(255) NOT NULL,
  type_name_zh character varying(255) NOT NULL,
  type_name_ko character varying(255) NOT NULL,
  color character varying(255) NOT NULL,
  direction integer DEFAULT 0 NOT NULL,
  kind integer DEFAULT 0 NOT NULL,
  top_priority integer DEFAULT 0 NOT NULL
);
ALTER TABLE stationapi.types OWNER TO stationapi;
--
-- Name: types_id_seq; Type: SEQUENCE; Schema: stationapi; Owner: stationapi
--

CREATE SEQUENCE stationapi.types_id_seq START WITH 1 INCREMENT BY 1 NO MINVALUE NO MAXVALUE CACHE 1;
ALTER SEQUENCE stationapi.types_id_seq OWNER TO stationapi;
--
-- Name: types_id_seq; Type: SEQUENCE OWNED BY; Schema: stationapi; Owner: stationapi
--

ALTER SEQUENCE stationapi.types_id_seq OWNED BY stationapi.types.id;
--
-- Name: aliases id; Type: DEFAULT; Schema: stationapi; Owner: stationapi
--

ALTER TABLE ONLY stationapi.aliases
ALTER COLUMN id
SET DEFAULT nextval('stationapi.aliases_id_seq'::regclass);
--
-- Name: line_aliases id; Type: DEFAULT; Schema: stationapi; Owner: stationapi
--

ALTER TABLE ONLY stationapi.line_aliases
ALTER COLUMN id
SET DEFAULT nextval('stationapi.line_aliases_id_seq'::regclass);
--
-- Name: station_station_types id; Type: DEFAULT; Schema: stationapi; Owner: stationapi
--

ALTER TABLE ONLY stationapi.station_station_types
ALTER COLUMN id
SET DEFAULT nextval(
    'stationapi.station_station_types_id_seq'::regclass
  );
--
-- Name: types id; Type: DEFAULT; Schema: stationapi; Owner: stationapi
--

ALTER TABLE ONLY stationapi.types
ALTER COLUMN id
SET DEFAULT nextval('stationapi.types_id_seq'::regclass);