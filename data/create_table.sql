PRAGMA journal_mode = MEMORY;
PRAGMA synchronous = OFF;
PRAGMA foreign_keys = OFF;
PRAGMA ignore_check_constraints = OFF;
PRAGMA auto_vacuum = NONE;
PRAGMA secure_delete = OFF;
BEGIN TRANSACTION;

DROP TABLE IF EXISTS `aliases`;

CREATE TABLE `aliases` (
`id` INTEGER NOT NULL,
`line_name` TEXT DEFAULT NULL,
`line_name_k` TEXT DEFAULT NULL,
`line_name_h` TEXT DEFAULT NULL,
`line_name_r` TEXT DEFAULT NULL,
`line_name_zh` TEXT DEFAULT NULL,
`line_name_ko` TEXT DEFAULT NULL,
`line_color_c` TEXT DEFAULT NULL,
PRIMARY KEY (`id`)
);
DROP TABLE IF EXISTS `companies`;

CREATE TABLE `companies` (
`company_cd` INTEGER NOT NULL,
`rr_cd` INTEGER NOT NULL,
`company_name` TEXT NOT NULL,
`company_name_k` TEXT NOT NULL,
`company_name_h` TEXT NOT NULL,
`company_name_r` TEXT NOT NULL,
`company_name_en` TEXT NOT NULL,
`company_name_full_en` TEXT NOT NULL,
`company_url` TEXT DEFAULT NULL,
`company_type` INTEGER NOT NULL,
`e_status` INTEGER NOT NULL,
`e_sort` INTEGER NOT NULL,
PRIMARY KEY (`company_cd`)
);
DROP TABLE IF EXISTS `connections`;

CREATE TABLE `connections` (
`id` INTEGER NOT NULL ,
`station_cd1` INTEGER NOT NULL,
`station_cd2` INTEGER NOT NULL,
`distance` REAL NOT NULL,
PRIMARY KEY (`id`)
);
DROP TABLE IF EXISTS `line_aliases`;

CREATE TABLE `line_aliases` (
`id` INTEGER PRIMARY KEY AUTOINCREMENT,
`station_cd` INTEGER NOT NULL,
`alias_cd` INTEGER NOT NULL,
FOREIGN KEY (`station_cd`) REFERENCES `stations` (`station_cd`),
FOREIGN KEY (`alias_cd`) REFERENCES `aliases` (`id`)
);
DROP TABLE IF EXISTS `lines`;

CREATE TABLE `lines` (
`line_cd` INTEGER NOT NULL,
`company_cd` INTEGER NOT NULL,
`line_name` TEXT DEFAULT '',
`line_name_k` TEXT DEFAULT '',
`line_name_h` TEXT DEFAULT '',
`line_name_r` TEXT DEFAULT '',
`line_name_rn` TEXT DEFAULT '',
`line_name_zh` TEXT DEFAULT '',
`line_name_ko` TEXT DEFAULT '',
`line_color_c` TEXT NOT NULL,
`line_type` INTEGER NOT NULL,
`line_symbol1` TEXT DEFAULT NULL,
`line_symbol2` TEXT DEFAULT NULL,
`line_symbol3` TEXT DEFAULT NULL,
`line_symbol4` TEXT DEFAULT NULL,
`line_symbol1_color` TEXT DEFAULT NULL,
`line_symbol2_color` TEXT DEFAULT NULL,
`line_symbol3_color` TEXT DEFAULT NULL,
`line_symbol4_color` TEXT DEFAULT NULL,
`line_symbol1_shape` TEXT DEFAULT NULL,
`line_symbol2_shape` TEXT DEFAULT NULL,
`line_symbol3_shape` TEXT DEFAULT NULL,
`line_symbol4_shape` TEXT DEFAULT NULL,
`e_status` INTEGER NOT NULL,
`e_sort` INTEGER NOT NULL,
`average_distance` REAL NOT NULL DEFAULT 0.0,
PRIMARY KEY (`line_cd`),
FOREIGN KEY (`company_cd`) REFERENCES `companies` (`company_cd`)
);
DROP TABLE IF EXISTS `station_station_types`;

CREATE TABLE `station_station_types` (
`id` INTEGER PRIMARY KEY AUTOINCREMENT,
`station_cd` INTEGER NOT NULL,
`type_cd` INTEGER NOT NULL,
`line_group_cd` INTEGER NOT NULL,
`pass` INTEGER NOT NULL DEFAULT 0,
FOREIGN KEY (`station_cd`) REFERENCES `stations` (`station_cd`),
FOREIGN KEY (`type_cd`) REFERENCES `types` (`type_cd`)
);
DROP TABLE IF EXISTS `stations`;

CREATE TABLE `stations` (
`station_cd` INTEGER NOT NULL,
`station_g_cd` INTEGER NOT NULL,
`station_name` TEXT NOT NULL,
`station_name_k` TEXT NOT NULL,
`station_name_r` TEXT DEFAULT NULL,
`station_name_rn` TEXT DEFAULT NULL,
`station_name_zh` TEXT DEFAULT NULL,
`station_name_ko` TEXT DEFAULT NULL,
`station_number1` TEXT DEFAULT NULL,
`station_number2` TEXT DEFAULT NULL,
`station_number3` TEXT DEFAULT NULL,
`station_number4` TEXT DEFAULT NULL,
`three_letter_code` TEXT DEFAULT NULL,
`line_cd` INTEGER NOT NULL,
`pref_cd` INTEGER NOT NULL,
`post` TEXT NOT NULL,
`address` TEXT NOT NULL,
`lon` REAL NOT NULL,
`lat` REAL NOT NULL,
`open_ymd` TEXT NOT NULL,
`close_ymd` TEXT NOT NULL,
`e_status` INTEGER NOT NULL,
`e_sort` INTEGER NOT NULL,
PRIMARY KEY (`station_cd`),
FOREIGN KEY (`line_cd`) REFERENCES `lines` (`line_cd`)
);
DROP TABLE IF EXISTS `types`;

CREATE TABLE `types` (
`id` INTEGER PRIMARY KEY AUTOINCREMENT,
`type_cd` INTEGER NOT NULL,
`type_name` TEXT NOT NULL,
`type_name_k` TEXT NOT NULL,
`type_name_r` TEXT NOT NULL,
`type_name_zh` TEXT NOT NULL,
`type_name_ko` TEXT NOT NULL,
`color` TEXT NOT NULL,
`direction` INTEGER NOT NULL DEFAULT 0,
`kind` INTEGER NOT NULL DEFAULT 0,
`priority` INTEGER NOT NULL DEFAULT 0
);
DROP TABLE IF EXISTS `station_rtree`;

CREATE VIRTUAL TABLE station_rtree USING rtree(
  station_cd,
  min_lat, max_lat,
  min_lon, max_lon
);


CREATE INDEX `connections_station_cd1` ON `connections` (`station_cd1`);
CREATE INDEX `connections_station_cd2` ON `connections` (`station_cd2`);
CREATE INDEX `connections_station_cd1_cd2` ON `connections` (`station_cd1`, `station_cd2`);
CREATE INDEX `companies_e_status` ON `companies` (`e_status`);
CREATE INDEX `lines_company_cd` ON `lines` (`company_cd`);
CREATE INDEX `lines_e_status` ON `lines` (`e_status`);
CREATE INDEX `station_station_types_type_cd` ON `station_station_types` (`type_cd`);
CREATE INDEX `station_station_types_station_cd` ON `station_station_types` (`station_cd`);
CREATE INDEX `station_station_types_line_group_cd` ON `station_station_types` (`line_group_cd`);
CREATE INDEX `station_station_types_station_type` ON `station_station_types` (`station_cd`, `type_cd`);
CREATE INDEX `station_station_types_line_group_type` ON `station_station_types` (`line_group_cd`, `type_cd`);
CREATE INDEX `stations_line_cd` ON `stations` (`line_cd`);
CREATE INDEX `stations_e_status` ON `stations` (`e_status`);
CREATE INDEX `stations_station_name` ON `stations` (`station_name`);
CREATE INDEX `stations_station_name_k` ON `stations` (`station_name_k`);
CREATE INDEX `stations_line_cd_e_status` ON `stations` (`line_cd`, `e_status`);
CREATE INDEX `station_station_types_station_pass_type` ON `station_station_types` (`station_cd`, `pass`, `type_cd`);
CREATE INDEX `station_station_types_linegroup_pass_type` ON `station_station_types` (`line_group_cd`, `pass`, `type_cd`);
CREATE INDEX `station_station_types_station_linegroup_pass` ON `station_station_types` (`station_cd`, `line_group_cd`, `pass`);
CREATE INDEX `types_priority_kind` ON `types` (`priority`, `kind`);
CREATE INDEX `types_type_priority_kind` ON `types` (`type_cd`, `priority`, `kind`);
CREATE INDEX `stations_line_status_sort` ON `stations` (`line_cd`, `e_status`, `e_sort`);
CREATE INDEX `stations_group_status_line` ON `stations` (`station_g_cd`, `e_status`, `line_cd`);
CREATE INDEX `stations_station_name_status` ON `stations` (`station_name`, `e_status`);
CREATE INDEX `stations_station_name_k_status` ON `stations` (`station_name_k`, `e_status`);
CREATE INDEX `lines_company_status` ON `lines` (`company_cd`, `e_status`);
CREATE INDEX `line_aliases_station_alias` ON `line_aliases` (`station_cd`, `alias_cd`);
CREATE INDEX `station_station_types_pass_linegroup` ON `station_station_types` (`pass`, `line_group_cd`);
CREATE INDEX `stations_lat_lon_status` ON `stations` (`lat`, `lon`, `e_status`);
ANALYZE;
CREATE INDEX `stations_name_group_status` ON `stations` (`station_name`, `station_g_cd`, `e_status`);
CREATE INDEX `stations_active_only` ON `stations` (`station_cd`) WHERE `e_status` = 0;
CREATE INDEX `lines_active_only` ON `lines` (`line_cd`) WHERE `e_status` = 0;
CREATE INDEX `station_station_types_active_only` ON `station_station_types` (`station_cd`, `type_cd`) WHERE `pass` <> 1; 

CREATE UNIQUE INDEX `types_type_cd` ON `types` (`type_cd`);

COMMIT;
