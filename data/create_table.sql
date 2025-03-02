-- MariaDB dump 10.19  Distrib 10.11.6-MariaDB, for debian-linux-gnu (aarch64)
--
-- Host: db    Database: stationapi
-- ------------------------------------------------------
-- Server version	11.5.2-MariaDB-ubu2404

/*!40101 SET @OLD_CHARACTER_SET_CLIENT=@@CHARACTER_SET_CLIENT */;
/*!40101 SET @OLD_CHARACTER_SET_RESULTS=@@CHARACTER_SET_RESULTS */;
/*!40101 SET @OLD_COLLATION_CONNECTION=@@COLLATION_CONNECTION */;
/*!40101 SET NAMES utf8mb4 */;
/*!40103 SET @OLD_TIME_ZONE=@@TIME_ZONE */;
/*!40103 SET TIME_ZONE='+00:00' */;
/*!40014 SET @OLD_UNIQUE_CHECKS=@@UNIQUE_CHECKS, UNIQUE_CHECKS=0 */;
/*!40014 SET @OLD_FOREIGN_KEY_CHECKS=@@FOREIGN_KEY_CHECKS, FOREIGN_KEY_CHECKS=0 */;
/*!40101 SET @OLD_SQL_MODE=@@SQL_MODE, SQL_MODE='NO_AUTO_VALUE_ON_ZERO' */;
/*!40111 SET @OLD_SQL_NOTES=@@SQL_NOTES, SQL_NOTES=0 */;

--
-- Table structure for table `aliases`
--

DROP TABLE IF EXISTS `aliases`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!40101 SET character_set_client = utf8 */;
CREATE TABLE `aliases` (
  `id` int(10) unsigned NOT NULL AUTO_INCREMENT,
  `line_name` varchar(255) DEFAULT NULL,
  `line_name_k` varchar(255) DEFAULT NULL,
  `line_name_h` varchar(255) DEFAULT NULL,
  `line_name_r` varchar(255) DEFAULT NULL,
  `line_name_zh` varchar(255) DEFAULT NULL,
  `line_name_ko` varchar(255) DEFAULT NULL,
  `line_color_c` varchar(255) DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB AUTO_INCREMENT=10 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Table structure for table `companies`
--

DROP TABLE IF EXISTS `companies`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!40101 SET character_set_client = utf8 */;
CREATE TABLE `companies` (
  `company_cd` int(10) unsigned NOT NULL,
  `rr_cd` int(10) unsigned NOT NULL,
  `company_name` varchar(255) NOT NULL,
  `company_name_k` varchar(255) NOT NULL,
  `company_name_h` varchar(255) NOT NULL,
  `company_name_r` varchar(255) NOT NULL,
  `company_name_en` varchar(255) NOT NULL,
  `company_name_full_en` varchar(255) NOT NULL,
  `company_url` varchar(255) DEFAULT NULL,
  `company_type` int(10) unsigned NOT NULL,
  `e_status` int(10) unsigned NOT NULL,
  `e_sort` int(10) unsigned NOT NULL,
  PRIMARY KEY (`company_cd`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Table structure for table `connections`
--

DROP TABLE IF EXISTS `connections`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!40101 SET character_set_client = utf8 */;
CREATE TABLE `connections` (
  `id` int(11) unsigned NOT NULL AUTO_INCREMENT,
  `station_cd1` int(11) unsigned NOT NULL,
  `station_cd2` int(11) unsigned NOT NULL,
  `distance` double unsigned NOT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB AUTO_INCREMENT=17665 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_uca1400_ai_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Table structure for table `line_aliases`
--

DROP TABLE IF EXISTS `line_aliases`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!40101 SET character_set_client = utf8 */;
CREATE TABLE `line_aliases` (
  `id` int(10) unsigned NOT NULL AUTO_INCREMENT,
  `station_cd` int(10) unsigned NOT NULL,
  `alias_cd` int(10) unsigned NOT NULL,
  PRIMARY KEY (`id`),
  KEY `station_cd` (`station_cd`),
  KEY `alias_cd` (`alias_cd`),
  CONSTRAINT `line_aliases_ibfk_1` FOREIGN KEY (`station_cd`) REFERENCES `stations` (`station_cd`),
  CONSTRAINT `line_aliases_ibfk_2` FOREIGN KEY (`alias_cd`) REFERENCES `aliases` (`id`)
) ENGINE=InnoDB AUTO_INCREMENT=87 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Table structure for table `lines`
--

DROP TABLE IF EXISTS `lines`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!40101 SET character_set_client = utf8 */;
CREATE TABLE `lines` (
  `line_cd` int(10) unsigned NOT NULL,
  `company_cd` int(10) unsigned NOT NULL,
  `line_name` varchar(255) NOT NULL,
  `line_name_k` varchar(255) NOT NULL,
  `line_name_h` varchar(255) NOT NULL,
  `line_name_r` varchar(255) NOT NULL,
  `line_name_zh` varchar(255) DEFAULT NULL,
  `line_name_ko` varchar(255) DEFAULT NULL,
  `line_color_c` varchar(255) NOT NULL,
  `line_type` int(10) unsigned NOT NULL,
  `line_symbol_primary` varchar(255) DEFAULT NULL,
  `line_symbol_secondary` varchar(255) DEFAULT NULL,
  `line_symbol_extra` varchar(255) DEFAULT NULL,
  `line_symbol_primary_color` varchar(255) DEFAULT NULL,
  `line_symbol_secondary_color` varchar(255) DEFAULT NULL,
  `line_symbol_extra_color` varchar(255) DEFAULT NULL,
  `line_symbol_primary_shape` varchar(255) DEFAULT NULL,
  `line_symbol_secondary_shape` varchar(255) DEFAULT NULL,
  `line_symbol_extra_shape` varchar(255) DEFAULT NULL,
  `e_status` int(10) unsigned NOT NULL,
  `e_sort` int(10) unsigned NOT NULL,
  `average_distance` double unsigned NOT NULL,
  PRIMARY KEY (`line_cd`),
  KEY `company_cd` (`company_cd`),
  KEY `e_sort` (`e_sort`),
  CONSTRAINT `lines_ibfk_1` FOREIGN KEY (`company_cd`) REFERENCES `companies` (`company_cd`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Table structure for table `station_station_types`
--

DROP TABLE IF EXISTS `station_station_types`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!40101 SET character_set_client = utf8 */;
CREATE TABLE `station_station_types` (
  `id` int(10) unsigned NOT NULL AUTO_INCREMENT,
  `station_cd` int(10) unsigned NOT NULL,
  `type_cd` int(10) unsigned NOT NULL,
  `line_group_cd` int(10) unsigned NOT NULL,
  `pass` int(10) unsigned NOT NULL DEFAULT 0,
  PRIMARY KEY (`id`),
  KEY `type_cd` (`type_cd`),
  KEY `station_cd` (`station_cd`),
  KEY `line_group_cd` (`line_group_cd`),
  CONSTRAINT `station_station_types_ibfk_1` FOREIGN KEY (`station_cd`) REFERENCES `stations` (`station_cd`),
  CONSTRAINT `station_station_types_ibfk_2` FOREIGN KEY (`type_cd`) REFERENCES `types` (`type_cd`)
) ENGINE=InnoDB AUTO_INCREMENT=39711 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Table structure for table `stations`
--

DROP TABLE IF EXISTS `stations`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!40101 SET character_set_client = utf8 */;
CREATE TABLE `stations` (
  `station_cd` int(10) unsigned NOT NULL,
  `station_g_cd` int(10) unsigned NOT NULL,
  `station_name` varchar(255) NOT NULL,
  `station_name_k` varchar(255) NOT NULL,
  `station_name_r` varchar(255) DEFAULT NULL,
  `station_name_zh` varchar(255) DEFAULT NULL,
  `station_name_ko` varchar(255) DEFAULT NULL,
  `primary_station_number` varchar(255) DEFAULT NULL,
  `secondary_station_number` varchar(255) DEFAULT NULL,
  `extra_station_number` varchar(255) DEFAULT NULL,
  `three_letter_code` varchar(255) DEFAULT NULL,
  `line_cd` int(10) unsigned NOT NULL,
  `pref_cd` int(10) unsigned NOT NULL,
  `post` varchar(255) NOT NULL,
  `address` varchar(255) NOT NULL,
  `lon` double unsigned NOT NULL,
  `lat` double unsigned NOT NULL,
  `open_ymd` varchar(255) NOT NULL,
  `close_ymd` varchar(255) NOT NULL,
  `e_status` int(10) unsigned NOT NULL,
  `e_sort` int(10) unsigned NOT NULL,
  PRIMARY KEY (`station_cd`),
  KEY `line_cd` (`line_cd`),
  KEY `station_g_cd` (`station_g_cd`),
  CONSTRAINT `stations_ibfk_1` FOREIGN KEY (`line_cd`) REFERENCES `lines` (`line_cd`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Table structure for table `types`
--

DROP TABLE IF EXISTS `types`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!40101 SET character_set_client = utf8 */;
CREATE TABLE `types` (
  `id` int(10) unsigned NOT NULL AUTO_INCREMENT,
  `type_cd` int(10) unsigned NOT NULL,
  `type_name` varchar(255) NOT NULL,
  `type_name_k` varchar(255) NOT NULL,
  `type_name_r` varchar(255) NOT NULL,
  `type_name_zh` varchar(255) NOT NULL,
  `type_name_ko` varchar(255) NOT NULL,
  `color` varchar(255) NOT NULL,
  `direction` int(10) unsigned NOT NULL DEFAULT 0,
  `kind` int(10) unsigned NOT NULL DEFAULT 0,
  `top_priority` int(10) unsigned NOT NULL DEFAULT 0,
  PRIMARY KEY (`id`),
  UNIQUE KEY `type_cd` (`type_cd`)
) ENGINE=InnoDB AUTO_INCREMENT=297 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

/*!40103 SET TIME_ZONE=@OLD_TIME_ZONE */;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;

-- Dump completed on 2025-03-02 11:52:52
