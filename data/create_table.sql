--
-- PostgreSQL database dump
--

-- Dumped from database version 17.4 (Debian 17.4-1.pgdg120+2)
-- Dumped by pg_dump version 17.4 (Homebrew)

SET statement_timeout = 0;

SET lock_timeout = 0;

SET idle_in_transaction_session_timeout = 0;

SET transaction_timeout = 0;

SET client_encoding = 'UTF8';

SET standard_conforming_strings = on;

SELECT pg_catalog.set_config ('search_path', '', false);

SET check_function_bodies = false;

SET xmloption = content;

SET client_min_messages = warning;

SET row_security = off;

DO $$
BEGIN
    BEGIN
        EXECUTE 'CREATE EXTENSION IF NOT EXISTS pg_trgm';
    EXCEPTION
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'pg_trgm extension could not be created due to insufficient privileges.';
        WHEN undefined_file THEN
            RAISE NOTICE 'pg_trgm extension not available on this server.';
    END;

    BEGIN
        EXECUTE 'CREATE EXTENSION IF NOT EXISTS btree_gist';
    EXCEPTION
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'btree_gist extension could not be created due to insufficient privileges.';
        WHEN undefined_file THEN
            RAISE NOTICE 'btree_gist extension not available on this server.';
    END;
END
$$;

ALTER TABLE IF EXISTS ONLY public.stations
DROP CONSTRAINT IF EXISTS stations_line_cd_fkey;

-- Drop GTFS foreign key constraints before dropping primary keys
ALTER TABLE IF EXISTS ONLY public.gtfs_stops
DROP CONSTRAINT IF EXISTS gtfs_stops_station_cd_fkey;

ALTER TABLE IF EXISTS ONLY public.gtfs_routes
DROP CONSTRAINT IF EXISTS gtfs_routes_line_cd_fkey;

ALTER TABLE IF EXISTS ONLY public.gtfs_routes
DROP CONSTRAINT IF EXISTS gtfs_routes_agency_id_fkey;

ALTER TABLE IF EXISTS ONLY public.gtfs_agencies
DROP CONSTRAINT IF EXISTS gtfs_agencies_company_cd_fkey;

ALTER TABLE IF EXISTS ONLY public.station_station_types
DROP CONSTRAINT IF EXISTS station_station_types_type_cd_fkey;

ALTER TABLE IF EXISTS ONLY public.station_station_types
DROP CONSTRAINT IF EXISTS station_station_types_station_cd_fkey;

ALTER TABLE IF EXISTS ONLY public.lines
DROP CONSTRAINT IF EXISTS lines_company_cd_fkey;

ALTER TABLE IF EXISTS ONLY public.line_aliases
DROP CONSTRAINT IF EXISTS line_aliases_station_cd_fkey;

ALTER TABLE IF EXISTS ONLY public.line_aliases
DROP CONSTRAINT IF EXISTS line_aliases_alias_cd_fkey;

DROP INDEX IF EXISTS public.idx_16432_types_type_cd;

DROP INDEX IF EXISTS public.idx_16426_stations_station_g_cd;

DROP INDEX IF EXISTS public.idx_16426_stations_line_cd;

DROP INDEX IF EXISTS public.idx_16426_stations_lat_lon;

DROP INDEX IF EXISTS public.idx_16426_stations_e_sort_station_cd;

DROP INDEX IF EXISTS public.idx_16421_station_station_types_type_cd;

DROP INDEX IF EXISTS public.idx_16421_station_station_types_station_cd;

DROP INDEX IF EXISTS public.idx_16421_station_station_types_line_group_cd;

DROP INDEX IF EXISTS public.idx_16407_lines_e_sort;

DROP INDEX IF EXISTS public.idx_16407_lines_company_cd;

DROP INDEX IF EXISTS public.idx_16403_line_aliases_station_cd;

DROP INDEX IF EXISTS public.idx_16403_line_aliases_alias_cd;

DROP INDEX IF EXISTS public.idx_performance_stations_point;

DROP INDEX IF EXISTS public.idx_performance_station_name_trgm;

DROP INDEX IF EXISTS public.idx_performance_station_name_k_trgm;

DROP INDEX IF EXISTS public.idx_performance_station_name_rn_trgm;

DROP INDEX IF EXISTS public.idx_performance_station_name_zh_trgm;

DROP INDEX IF EXISTS public.idx_performance_station_name_ko_trgm;

ALTER TABLE IF EXISTS ONLY public.types
DROP CONSTRAINT IF EXISTS idx_16432_types_pkey;

ALTER TABLE IF EXISTS ONLY public.stations
DROP CONSTRAINT IF EXISTS idx_16426_stations_pkey;

ALTER TABLE IF EXISTS ONLY public.station_station_types
DROP CONSTRAINT IF EXISTS idx_16421_station_station_types_pkey;

ALTER TABLE IF EXISTS ONLY public.lines
DROP CONSTRAINT IF EXISTS idx_16407_lines_pkey;

ALTER TABLE IF EXISTS ONLY public.line_aliases
DROP CONSTRAINT IF EXISTS idx_16403_line_aliases_pkey;

ALTER TABLE IF EXISTS ONLY public.connections
DROP CONSTRAINT IF EXISTS idx_16399_connections_pkey;

ALTER TABLE IF EXISTS ONLY public.companies
DROP CONSTRAINT IF EXISTS idx_16394_companies_pkey;

ALTER TABLE IF EXISTS ONLY public.aliases
DROP CONSTRAINT IF EXISTS idx_16389_aliases_pkey;

ALTER TABLE IF EXISTS public.types ALTER COLUMN id DROP DEFAULT;

ALTER TABLE IF EXISTS public.station_station_types
ALTER COLUMN id
DROP DEFAULT;

ALTER TABLE IF EXISTS public.line_aliases
ALTER COLUMN id
DROP DEFAULT;

DROP SEQUENCE IF EXISTS public.types_id_seq;

DROP TABLE IF EXISTS public.types;

DROP TABLE IF EXISTS public.stations;

DROP TABLE IF EXISTS public.station_station_types;

DROP TABLE IF EXISTS public.lines;

DROP SEQUENCE IF EXISTS public.line_aliases_id_seq;

DROP TABLE IF EXISTS public.line_aliases;

DROP TABLE IF EXISTS public.connections;

DROP TABLE IF EXISTS public.companies;

DROP TABLE IF EXISTS public.aliases;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: aliases; Type: TABLE; Schema: public; Owner: stationapi
--

CREATE UNLOGGED TABLE public.aliases (
    id integer NOT NULL,
    line_name text,
    line_name_k text,
    line_name_h text,
    line_name_r text,
    line_name_zh text,
    line_name_ko text,
    line_color_c text
);

ALTER TABLE public.aliases OWNER TO stationapi;

--
-- Name: companies; Type: TABLE; Schema: public; Owner: stationapi
--

CREATE UNLOGGED TABLE public.companies (
    company_cd integer NOT NULL,
    rr_cd integer NOT NULL,
    company_name text NOT NULL,
    company_name_k text NOT NULL,
    company_name_h text NOT NULL,
    company_name_r text NOT NULL,
    company_name_en text NOT NULL,
    company_name_full_en text NOT NULL,
    company_url text,
    company_type integer NOT NULL,
    e_status integer NOT NULL,
    e_sort integer NOT NULL
);

ALTER TABLE public.companies OWNER TO stationapi;

--
-- Name: connections; Type: TABLE; Schema: public; Owner: stationapi
--

CREATE UNLOGGED TABLE public.connections (
    id integer NOT NULL,
    station_cd1 integer NOT NULL,
    station_cd2 integer NOT NULL,
    distance real DEFAULT '0'::real
);

ALTER TABLE public.connections OWNER TO stationapi;

--
-- Name: line_aliases; Type: TABLE; Schema: public; Owner: stationapi
--

CREATE UNLOGGED TABLE public.line_aliases (
    id integer NOT NULL,
    station_cd integer NOT NULL,
    alias_cd integer NOT NULL
);

ALTER TABLE public.line_aliases OWNER TO stationapi;

--
-- Name: line_aliases_id_seq; Type: SEQUENCE; Schema: public; Owner: stationapi
--

CREATE SEQUENCE public.line_aliases_id_seq START
WITH
    1 INCREMENT BY 1 NO MINVALUE NO MAXVALUE CACHE 1;

ALTER SEQUENCE public.line_aliases_id_seq OWNER TO stationapi;

--
-- Name: line_aliases_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: stationapi
--

ALTER SEQUENCE public.line_aliases_id_seq OWNED BY public.line_aliases.id;

--
-- Name: lines; Type: TABLE; Schema: public; Owner: stationapi
--

CREATE UNLOGGED TABLE public.lines (
    line_cd integer NOT NULL,
    company_cd integer NOT NULL,
    line_name text NOT NULL,
    line_name_k text NOT NULL,
    line_name_h text NOT NULL,
    line_name_r text NOT NULL DEFAULT ''::text,
    line_name_rn text NOT NULL DEFAULT ''::text,
    line_name_zh text DEFAULT ''::text,
    line_name_ko text DEFAULT ''::text,
    line_color_c text NOT NULL,
    line_type integer NOT NULL,
    line_symbol1 text,
    line_symbol2 text,
    line_symbol3 text,
    line_symbol4 text,
    line_symbol1_color text,
    line_symbol2_color text,
    line_symbol3_color text,
    line_symbol4_color text,
    line_symbol1_shape text,
    line_symbol2_shape text,
    line_symbol3_shape text,
    line_symbol4_shape text,
    e_status integer NOT NULL,
    e_sort integer NOT NULL,
    average_distance real DEFAULT '0'::real
);

ALTER TABLE public.lines OWNER TO stationapi;

--
-- Name: station_station_types; Type: TABLE; Schema: public; Owner: stationapi
--

CREATE UNLOGGED TABLE public.station_station_types (
    id SERIAL NOT NULL,
    station_cd integer NOT NULL,
    type_cd integer NOT NULL,
    line_group_cd integer NOT NULL,
    pass integer DEFAULT '0'::integer
);

ALTER TABLE public.station_station_types OWNER TO stationapi;

--
-- Name: stations; Type: TABLE; Schema: public; Owner: stationapi
--

CREATE UNLOGGED TABLE public.stations (
    station_cd integer NOT NULL,
    station_g_cd integer NOT NULL,
    station_name text NOT NULL,
    station_name_k text NOT NULL,
    station_name_r text,
    station_name_rn text,
    station_name_zh text,
    station_name_ko text,
    station_number1 text,
    station_number2 text,
    station_number3 text,
    station_number4 text,
    three_letter_code text,
    line_cd integer NOT NULL,
    pref_cd integer NOT NULL,
    post text NOT NULL,
    address text NOT NULL,
    lon DOUBLE PRECISION NOT NULL,
    lat DOUBLE PRECISION NOT NULL,
    open_ymd text NOT NULL,
    close_ymd text NOT NULL,
    e_status integer NOT NULL,
    e_sort integer NOT NULL
);

ALTER TABLE public.stations OWNER TO stationapi;

--
-- Name: types; Type: TABLE; Schema: public; Owner: stationapi
--

CREATE UNLOGGED TABLE public.types (
    id integer NOT NULL,
    type_cd integer NOT NULL,
    type_name text NOT NULL,
    type_name_k text NOT NULL,
    type_name_r text NOT NULL,
    type_name_zh text NOT NULL,
    type_name_ko text NOT NULL,
    color text NOT NULL,
    direction integer DEFAULT '0'::integer,
    kind integer DEFAULT '0'::integer,
    priority integer NOT NULL DEFAULT '0'::integer
);

ALTER TABLE public.types OWNER TO stationapi;

--
-- Name: types_id_seq; Type: SEQUENCE; Schema: public; Owner: stationapi
--

CREATE SEQUENCE public.types_id_seq START
WITH
    1 INCREMENT BY 1 NO MINVALUE NO MAXVALUE CACHE 1;

ALTER SEQUENCE public.types_id_seq OWNER TO stationapi;

--
-- Name: types_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: stationapi
--

ALTER SEQUENCE public.types_id_seq OWNED BY public.types.id;

--
-- Name: line_aliases id; Type: DEFAULT; Schema: public; Owner: stationapi
--

ALTER TABLE ONLY public.line_aliases
ALTER COLUMN id
SET DEFAULT nextval(
    'public.line_aliases_id_seq'::regclass
);

--
-- Name: types id; Type: DEFAULT; Schema: public; Owner: stationapi
--

ALTER TABLE ONLY public.types
ALTER COLUMN id
SET DEFAULT nextval(
    'public.types_id_seq'::regclass
);

--
-- Name: aliases idx_16389_aliases_pkey; Type: CONSTRAINT; Schema: public; Owner: stationapi
--

ALTER TABLE ONLY public.aliases
ADD CONSTRAINT idx_16389_aliases_pkey PRIMARY KEY (id);

--
-- Name: companies idx_16394_companies_pkey; Type: CONSTRAINT; Schema: public; Owner: stationapi
--

ALTER TABLE ONLY public.companies
ADD CONSTRAINT idx_16394_companies_pkey PRIMARY KEY (company_cd);

--
-- Name: lines idx_16407_lines_pkey; Type: CONSTRAINT; Schema: public; Owner: stationapi
--

ALTER TABLE ONLY public.lines
ADD CONSTRAINT idx_16407_lines_pkey PRIMARY KEY (line_cd);

--
-- Name: stations idx_16426_stations_pkey; Type: CONSTRAINT; Schema: public; Owner: stationapi
--

ALTER TABLE ONLY public.stations
ADD CONSTRAINT idx_16426_stations_pkey PRIMARY KEY (station_cd);

--
-- Name: idx_16403_line_aliases_alias_cd; Type: INDEX; Schema: public; Owner: stationapi
--

CREATE INDEX idx_16403_line_aliases_alias_cd ON public.line_aliases USING btree (alias_cd);

--
-- Name: idx_16403_line_aliases_station_cd; Type: INDEX; Schema: public; Owner: stationapi
--

CREATE INDEX idx_16403_line_aliases_station_cd ON public.line_aliases USING btree (station_cd);

--
-- Name: idx_16407_lines_company_cd; Type: INDEX; Schema: public; Owner: stationapi
--

CREATE INDEX idx_16407_lines_company_cd ON public.lines USING btree (company_cd);

--
-- Name: idx_16407_lines_e_sort; Type: INDEX; Schema: public; Owner: stationapi
--

CREATE INDEX idx_16407_lines_e_sort ON public.lines USING btree (e_sort);

--
-- Name: idx_16421_station_station_types_line_group_cd; Type: INDEX; Schema: public; Owner: stationapi
--

CREATE INDEX idx_16421_station_station_types_line_group_cd ON public.station_station_types USING btree (line_group_cd);

--
-- Name: idx_16421_station_station_types_station_cd; Type: INDEX; Schema: public; Owner: stationapi
--

CREATE INDEX idx_16421_station_station_types_station_cd ON public.station_station_types USING btree (station_cd);

--
-- Name: idx_16421_station_station_types_type_cd; Type: INDEX; Schema: public; Owner: stationapi
--

CREATE INDEX idx_16421_station_station_types_type_cd ON public.station_station_types USING btree (type_cd);

--
-- Name: idx_16426_stations_e_sort_station_cd; Type: INDEX; Schema: public; Owner: stationapi
--

CREATE INDEX idx_16426_stations_e_sort_station_cd ON public.stations USING btree (e_sort, station_cd);

--
-- Name: idx_16426_stations_lat_lon; Type: INDEX; Schema: public; Owner: stationapi
--

CREATE INDEX idx_16426_stations_lat_lon ON public.stations USING btree (lat, lon);

--
-- Name: idx_16426_stations_line_cd; Type: INDEX; Schema: public; Owner: stationapi
--

CREATE INDEX idx_16426_stations_line_cd ON public.stations USING btree (line_cd);

--
-- Name: idx_16426_stations_station_g_cd; Type: INDEX; Schema: public; Owner: stationapi
--

CREATE INDEX idx_16426_stations_station_g_cd ON public.stations USING btree (station_g_cd);

DO $$
BEGIN
    BEGIN
        EXECUTE 'CREATE INDEX IF NOT EXISTS idx_performance_stations_point ON public.stations USING gist ((point(lat, lon)))';
    EXCEPTION
        WHEN undefined_object THEN
            RAISE NOTICE 'Skipping GiST point index; required operator class is unavailable.';
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'Skipping GiST point index; insufficient privileges.';
    END;
END
$$;

DO $$
BEGIN
    BEGIN
        EXECUTE 'CREATE INDEX IF NOT EXISTS idx_performance_station_name_trgm ON public.stations USING gin (station_name gin_trgm_ops)';
        EXECUTE 'CREATE INDEX IF NOT EXISTS idx_performance_station_name_k_trgm ON public.stations USING gin (station_name_k gin_trgm_ops)';
        EXECUTE 'CREATE INDEX IF NOT EXISTS idx_performance_station_name_rn_trgm ON public.stations USING gin (station_name_rn gin_trgm_ops)';
        EXECUTE 'CREATE INDEX IF NOT EXISTS idx_performance_station_name_zh_trgm ON public.stations USING gin (station_name_zh gin_trgm_ops)';
        EXECUTE 'CREATE INDEX IF NOT EXISTS idx_performance_station_name_ko_trgm ON public.stations USING gin (station_name_ko gin_trgm_ops)';
    EXCEPTION
        WHEN undefined_object THEN
            RAISE NOTICE 'Skipping trigram GIN indexes; gin_trgm_ops operator class is unavailable.';
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'Skipping trigram GIN indexes; insufficient privileges.';
    END;
END
$$;

--
-- Name: idx_16432_types_type_cd; Type: INDEX; Schema: public; Owner: stationapi
--

CREATE UNIQUE INDEX idx_16432_types_type_cd ON public.types USING btree (type_cd);

--
-- Name: line_aliases line_aliases_alias_cd_fkey; Type: FK CONSTRAINT; Schema: public; Owner: stationapi
--

ALTER TABLE ONLY public.line_aliases
ADD CONSTRAINT line_aliases_alias_cd_fkey FOREIGN KEY (alias_cd) REFERENCES public.aliases (id);

--
-- Name: line_aliases line_aliases_station_cd_fkey; Type: FK CONSTRAINT; Schema: public; Owner: stationapi
--

ALTER TABLE ONLY public.line_aliases
ADD CONSTRAINT line_aliases_station_cd_fkey FOREIGN KEY (station_cd) REFERENCES public.stations (station_cd);

--
-- Name: lines lines_company_cd_fkey; Type: FK CONSTRAINT; Schema: public; Owner: stationapi
--

ALTER TABLE ONLY public.lines
ADD CONSTRAINT lines_company_cd_fkey FOREIGN KEY (company_cd) REFERENCES public.companies (company_cd);

--
-- Name: station_station_types station_station_types_station_cd_fkey; Type: FK CONSTRAINT; Schema: public; Owner: stationapi
--

ALTER TABLE ONLY public.station_station_types
ADD CONSTRAINT station_station_types_station_cd_fkey FOREIGN KEY (station_cd) REFERENCES public.stations (station_cd);

--
-- Name: station_station_types station_station_types_type_cd_fkey; Type: FK CONSTRAINT; Schema: public; Owner: stationapi
--

ALTER TABLE ONLY public.station_station_types
ADD CONSTRAINT station_station_types_type_cd_fkey FOREIGN KEY (type_cd) REFERENCES public.types (type_cd);

--
-- Name: stations stations_line_cd_fkey; Type: FK CONSTRAINT; Schema: public; Owner: stationapi
--

ALTER TABLE ONLY public.stations
ADD CONSTRAINT stations_line_cd_fkey FOREIGN KEY (line_cd) REFERENCES public.lines (line_cd);

--
-- PostgreSQL database dump complete
--

-- ============================================================
-- GTFS Bus Integration - Phase 1: Schema Extensions
-- ============================================================

--
-- Add transport_type to stations table (0: Rail, 1: Bus)
--

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'stations' AND column_name = 'transport_type'
    ) THEN
        ALTER TABLE public.stations ADD COLUMN transport_type INTEGER DEFAULT 0 NOT NULL;
    END IF;
END $$;

--
-- Add transport_type to lines table (0: Rail, 1: Bus)
--

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'lines' AND column_name = 'transport_type'
    ) THEN
        ALTER TABLE public.lines ADD COLUMN transport_type INTEGER DEFAULT 0 NOT NULL;
    END IF;
END $$;

--
-- Create index for transport_type filtering
--

CREATE INDEX IF NOT EXISTS idx_stations_transport_type ON public.stations USING btree (transport_type);
CREATE INDEX IF NOT EXISTS idx_lines_transport_type ON public.lines USING btree (transport_type);

-- ============================================================
-- GTFS Tables
-- ============================================================

--
-- Name: gtfs_agencies; Type: TABLE; Schema: public
-- GTFS agency information (bus operators)
--

DROP TABLE IF EXISTS public.gtfs_stop_times CASCADE;
DROP TABLE IF EXISTS public.gtfs_trips CASCADE;
DROP TABLE IF EXISTS public.gtfs_calendar_dates CASCADE;
DROP TABLE IF EXISTS public.gtfs_calendar CASCADE;
DROP TABLE IF EXISTS public.gtfs_stops CASCADE;
DROP TABLE IF EXISTS public.gtfs_routes CASCADE;
DROP TABLE IF EXISTS public.gtfs_agencies CASCADE;
DROP TABLE IF EXISTS public.gtfs_shapes CASCADE;
DROP TABLE IF EXISTS public.gtfs_feed_info CASCADE;

CREATE UNLOGGED TABLE public.gtfs_agencies (
    agency_id VARCHAR(255) PRIMARY KEY,
    agency_name TEXT NOT NULL,
    agency_name_k TEXT,
    agency_name_r TEXT,
    agency_name_zh TEXT,
    agency_name_ko TEXT,
    agency_url TEXT,
    agency_timezone VARCHAR(50) DEFAULT 'Asia/Tokyo',
    agency_lang VARCHAR(10) DEFAULT 'ja',
    agency_phone TEXT,
    agency_fare_url TEXT,
    company_cd INTEGER REFERENCES public.companies(company_cd)
);

ALTER TABLE public.gtfs_agencies OWNER TO stationapi;

--
-- Name: gtfs_routes; Type: TABLE; Schema: public
-- GTFS route information (bus lines and rail lines from GTFS sources)
--

CREATE UNLOGGED TABLE public.gtfs_routes (
    route_id VARCHAR(255) PRIMARY KEY,
    agency_id VARCHAR(255) REFERENCES public.gtfs_agencies(agency_id),
    route_short_name TEXT,
    route_long_name TEXT,
    route_long_name_k TEXT,
    route_long_name_r TEXT,
    route_long_name_zh TEXT,
    route_long_name_ko TEXT,
    route_desc TEXT,
    route_type INTEGER NOT NULL DEFAULT 3,  -- 3 = Bus, 1 = Subway, 2 = Rail
    route_url TEXT,
    route_color VARCHAR(6),
    route_text_color VARCHAR(6),
    route_sort_order INTEGER,
    line_cd INTEGER REFERENCES public.lines(line_cd),
    transport_type INTEGER DEFAULT 1,  -- 1: Bus, 2: Rail (GTFS)
    company_cd INTEGER REFERENCES public.companies(company_cd),
    source_id VARCHAR(50)  -- GTFS source identifier (e.g., "toei_bus", "tokyo_metro")
);

ALTER TABLE public.gtfs_routes OWNER TO stationapi;

CREATE INDEX idx_gtfs_routes_agency_id ON public.gtfs_routes USING btree (agency_id);
CREATE INDEX idx_gtfs_routes_line_cd ON public.gtfs_routes USING btree (line_cd);
CREATE INDEX idx_gtfs_routes_transport_type ON public.gtfs_routes USING btree (transport_type);
CREATE INDEX idx_gtfs_routes_source_id ON public.gtfs_routes USING btree (source_id);

--
-- Name: gtfs_stops; Type: TABLE; Schema: public
-- GTFS stop information (bus stops)
--

CREATE UNLOGGED TABLE public.gtfs_stops (
    stop_id VARCHAR(255) PRIMARY KEY,
    stop_code VARCHAR(50),
    stop_name TEXT NOT NULL,
    stop_name_k TEXT,
    stop_name_r TEXT,
    stop_name_zh TEXT,
    stop_name_ko TEXT,
    stop_desc TEXT,
    stop_lat DOUBLE PRECISION NOT NULL,
    stop_lon DOUBLE PRECISION NOT NULL,
    zone_id VARCHAR(255),
    stop_url TEXT,
    location_type INTEGER DEFAULT 0,  -- 0: stop, 1: station
    parent_station VARCHAR(255),
    stop_timezone VARCHAR(50),
    wheelchair_boarding INTEGER,
    platform_code VARCHAR(50),
    station_cd INTEGER REFERENCES public.stations(station_cd)
);

ALTER TABLE public.gtfs_stops OWNER TO stationapi;

CREATE INDEX idx_gtfs_stops_station_cd ON public.gtfs_stops USING btree (station_cd);
CREATE INDEX idx_gtfs_stops_parent_station ON public.gtfs_stops USING btree (parent_station);

DO $$
BEGIN
    BEGIN
        EXECUTE 'CREATE INDEX IF NOT EXISTS idx_gtfs_stops_point ON public.gtfs_stops USING gist ((point(stop_lat, stop_lon)))';
    EXCEPTION
        WHEN undefined_object THEN
            RAISE NOTICE 'Skipping GiST point index for gtfs_stops; required operator class is unavailable.';
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'Skipping GiST point index for gtfs_stops; insufficient privileges.';
    END;
END $$;

DO $$
BEGIN
    BEGIN
        EXECUTE 'CREATE INDEX IF NOT EXISTS idx_gtfs_stops_name_trgm ON public.gtfs_stops USING gin (stop_name gin_trgm_ops)';
        EXECUTE 'CREATE INDEX IF NOT EXISTS idx_gtfs_stops_name_k_trgm ON public.gtfs_stops USING gin (stop_name_k gin_trgm_ops)';
    EXCEPTION
        WHEN undefined_object THEN
            RAISE NOTICE 'Skipping trigram GIN indexes for gtfs_stops; gin_trgm_ops operator class is unavailable.';
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'Skipping trigram GIN indexes for gtfs_stops; insufficient privileges.';
    END;
END $$;

--
-- Name: gtfs_calendar; Type: TABLE; Schema: public
-- GTFS calendar (service schedules)
--

CREATE UNLOGGED TABLE public.gtfs_calendar (
    service_id VARCHAR(255) PRIMARY KEY,
    monday BOOLEAN NOT NULL DEFAULT FALSE,
    tuesday BOOLEAN NOT NULL DEFAULT FALSE,
    wednesday BOOLEAN NOT NULL DEFAULT FALSE,
    thursday BOOLEAN NOT NULL DEFAULT FALSE,
    friday BOOLEAN NOT NULL DEFAULT FALSE,
    saturday BOOLEAN NOT NULL DEFAULT FALSE,
    sunday BOOLEAN NOT NULL DEFAULT FALSE,
    start_date DATE NOT NULL,
    end_date DATE NOT NULL
);

ALTER TABLE public.gtfs_calendar OWNER TO stationapi;

--
-- Name: gtfs_calendar_dates; Type: TABLE; Schema: public
-- GTFS calendar dates (service exceptions)
--

CREATE UNLOGGED TABLE public.gtfs_calendar_dates (
    id SERIAL PRIMARY KEY,
    service_id VARCHAR(255) NOT NULL,
    date DATE NOT NULL,
    exception_type INTEGER NOT NULL  -- 1: added, 2: removed
);

ALTER TABLE public.gtfs_calendar_dates OWNER TO stationapi;

CREATE INDEX idx_gtfs_calendar_dates_service_id ON public.gtfs_calendar_dates USING btree (service_id);
CREATE INDEX idx_gtfs_calendar_dates_date ON public.gtfs_calendar_dates USING btree (date);

--
-- Name: gtfs_trips; Type: TABLE; Schema: public
-- GTFS trip information
--

CREATE UNLOGGED TABLE public.gtfs_trips (
    trip_id VARCHAR(255) PRIMARY KEY,
    route_id VARCHAR(255) NOT NULL REFERENCES public.gtfs_routes(route_id),
    service_id VARCHAR(255) NOT NULL,
    trip_headsign TEXT,
    trip_headsign_k TEXT,
    trip_headsign_r TEXT,
    trip_short_name TEXT,
    direction_id INTEGER,  -- 0: outbound, 1: inbound
    block_id VARCHAR(255),
    shape_id VARCHAR(255),
    wheelchair_accessible INTEGER,
    bikes_allowed INTEGER
);

ALTER TABLE public.gtfs_trips OWNER TO stationapi;

CREATE INDEX idx_gtfs_trips_route_id ON public.gtfs_trips USING btree (route_id);
CREATE INDEX idx_gtfs_trips_service_id ON public.gtfs_trips USING btree (service_id);
CREATE INDEX idx_gtfs_trips_shape_id ON public.gtfs_trips USING btree (shape_id);

--
-- Name: gtfs_stop_times; Type: TABLE; Schema: public
-- GTFS stop times (timetable)
--

CREATE UNLOGGED TABLE public.gtfs_stop_times (
    id SERIAL PRIMARY KEY,
    trip_id VARCHAR(255) NOT NULL REFERENCES public.gtfs_trips(trip_id),
    arrival_time TEXT,  -- GTFS allows times > 24:00 (e.g., "25:30:00")
    departure_time TEXT,
    stop_id VARCHAR(255) NOT NULL REFERENCES public.gtfs_stops(stop_id),
    stop_sequence INTEGER NOT NULL,
    stop_headsign TEXT,
    pickup_type INTEGER DEFAULT 0,
    drop_off_type INTEGER DEFAULT 0,
    shape_dist_traveled DOUBLE PRECISION,
    timepoint INTEGER DEFAULT 1
);

ALTER TABLE public.gtfs_stop_times OWNER TO stationapi;

CREATE INDEX idx_gtfs_stop_times_trip_id ON public.gtfs_stop_times USING btree (trip_id);
CREATE INDEX idx_gtfs_stop_times_stop_id ON public.gtfs_stop_times USING btree (stop_id);
CREATE INDEX idx_gtfs_stop_times_arrival_time ON public.gtfs_stop_times USING btree (arrival_time);
CREATE UNIQUE INDEX idx_gtfs_stop_times_trip_stop_seq ON public.gtfs_stop_times USING btree (trip_id, stop_sequence);

--
-- Name: gtfs_shapes; Type: TABLE; Schema: public
-- GTFS shapes (route geometry)
--

CREATE UNLOGGED TABLE public.gtfs_shapes (
    id SERIAL PRIMARY KEY,
    shape_id VARCHAR(255) NOT NULL,
    shape_pt_lat DOUBLE PRECISION NOT NULL,
    shape_pt_lon DOUBLE PRECISION NOT NULL,
    shape_pt_sequence INTEGER NOT NULL,
    shape_dist_traveled DOUBLE PRECISION
);

ALTER TABLE public.gtfs_shapes OWNER TO stationapi;

CREATE INDEX idx_gtfs_shapes_shape_id ON public.gtfs_shapes USING btree (shape_id);
CREATE UNIQUE INDEX idx_gtfs_shapes_id_seq ON public.gtfs_shapes USING btree (shape_id, shape_pt_sequence);

--
-- Name: gtfs_feed_info; Type: TABLE; Schema: public
-- GTFS feed metadata
--

CREATE UNLOGGED TABLE public.gtfs_feed_info (
    id SERIAL PRIMARY KEY,
    feed_publisher_name TEXT NOT NULL,
    feed_publisher_url TEXT,
    feed_lang VARCHAR(10) DEFAULT 'ja',
    feed_start_date DATE,
    feed_end_date DATE,
    feed_version TEXT,
    feed_contact_email TEXT,
    feed_contact_url TEXT,
    imported_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE public.gtfs_feed_info OWNER TO stationapi;

-- ============================================================
-- End of GTFS Bus Integration Schema
-- ============================================================

-- ============================================================
-- Stop Pattern Detection Schema
-- For detecting train type stop pattern changes from ODPT API
-- ============================================================

--
-- Name: stop_pattern_snapshots; Type: TABLE; Schema: public
-- Snapshots of stop patterns extracted from ODPT TrainTimetable
--

CREATE TABLE public.stop_pattern_snapshots (
    id SERIAL PRIMARY KEY,
    operator_id VARCHAR(100) NOT NULL,     -- odpt.Operator:TokyoMetro
    railway_id VARCHAR(100) NOT NULL,      -- odpt.Railway:TokyoMetro.Marunouchi
    train_type_id VARCHAR(100) NOT NULL,   -- odpt.TrainType:TokyoMetro.Local
    train_type_name VARCHAR(100),          -- 各停
    station_ids TEXT[] NOT NULL,           -- Array of station IDs
    station_names TEXT[],                  -- Array of station names (for reference)
    captured_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(railway_id, train_type_id, (captured_at::date))
);

ALTER TABLE public.stop_pattern_snapshots OWNER TO stationapi;

CREATE INDEX idx_stop_pattern_snapshots_railway ON public.stop_pattern_snapshots USING btree (railway_id, train_type_id);
CREATE INDEX idx_stop_pattern_snapshots_operator ON public.stop_pattern_snapshots USING btree (operator_id);
CREATE INDEX idx_stop_pattern_snapshots_captured ON public.stop_pattern_snapshots USING btree (captured_at DESC);

--
-- Name: stop_pattern_changes; Type: TABLE; Schema: public
-- Log of detected stop pattern changes
--

CREATE TABLE public.stop_pattern_changes (
    id SERIAL PRIMARY KEY,
    operator_id VARCHAR(100) NOT NULL,
    railway_id VARCHAR(100) NOT NULL,
    railway_name VARCHAR(100),             -- 丸ノ内線
    train_type_id VARCHAR(100) NOT NULL,
    train_type_name VARCHAR(100),          -- 各停
    change_type VARCHAR(20) NOT NULL,      -- 'added' or 'removed'
    station_id VARCHAR(100) NOT NULL,
    station_name VARCHAR(100),
    detected_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    acknowledged BOOLEAN DEFAULT FALSE,
    acknowledged_at TIMESTAMP
);

ALTER TABLE public.stop_pattern_changes OWNER TO stationapi;

CREATE INDEX idx_stop_pattern_changes_detected ON public.stop_pattern_changes USING btree (detected_at DESC);
CREATE INDEX idx_stop_pattern_changes_unack ON public.stop_pattern_changes (acknowledged) WHERE acknowledged = FALSE;
CREATE INDEX idx_stop_pattern_changes_railway ON public.stop_pattern_changes USING btree (railway_id, train_type_id);

-- ============================================================
-- End of Stop Pattern Detection Schema
-- ============================================================
