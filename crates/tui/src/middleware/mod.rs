pub mod config;
pub mod discovery;
pub mod redis;
pub mod es;
pub mod kafka;
pub mod nginx;
pub mod tomcat;
pub mod caddy;
pub mod docker;

use ratatui::style::Color;
use crate::theme::Theme;
use discovery::DiscoveredService;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MiddlewareKind {
    Overview, Redis, Elasticsearch, Kafka, Nginx, Tomcat, Caddy,
    Mysql, Postgresql, Mongodb, Rabbitmq, Haproxy, Keepalived,
    Etcd, Consul, Zookeeper, Minio, Prometheus, Grafana, Memcached, Docker,
}

impl MiddlewareKind {
    pub fn name(&self) -> &str {
        match self {
            Self::Overview=>"总览",Self::Redis=>"Redis",Self::Elasticsearch=>"ES",
            Self::Kafka=>"Kafka",Self::Nginx=>"Nginx",Self::Tomcat=>"Tomcat",
            Self::Caddy=>"Caddy",Self::Mysql=>"MySQL",Self::Postgresql=>"PostgreSQL",
            Self::Mongodb=>"MongoDB",Self::Rabbitmq=>"RabbitMQ",Self::Haproxy=>"HAProxy",
            Self::Keepalived=>"Keepalived",Self::Etcd=>"Etcd",Self::Consul=>"Consul",
            Self::Zookeeper=>"ZK",Self::Minio=>"MinIO",Self::Prometheus=>"Prometheus",
            Self::Grafana=>"Grafana",Self::Memcached=>"Memcached",Self::Docker=>"Docker",
        }
    }
    pub fn icon(&self) -> &str {
        match self {
            Self::Overview=>"\u{2630}",Self::Redis=>"\u{25cf}",Self::Elasticsearch=>"\u{25ce}",
            Self::Kafka=>"\u{25c6}",Self::Nginx=>"\u{25c8}",Self::Tomcat=>"\u{25b2}",Self::Caddy=>"\u{2605}",
            Self::Mysql=>"\u{25c7}",Self::Postgresql=>"\u{25a3}",Self::Mongodb=>"\u{25c9}",
            Self::Rabbitmq=>"\u{25d7}",Self::Haproxy=>"\u{21cb}",Self::Keepalived=>"\u{271a}",
            Self::Etcd=>"\u{2261}",Self::Consul=>"\u{2299}",Self::Zookeeper=>"\u{2117}",Self::Minio=>"\u{21a6}",
            Self::Prometheus=>"\u{2606}",Self::Grafana=>"\u{2237}",Self::Memcached=>"\u{25a1}",Self::Docker=>"\u{25a3}",
        }
    }
    pub fn color(&self, t:&Theme)->Color {
        match self {
            Self::Overview=>t.text,Self::Redis=>t.error,Self::Elasticsearch=>t.warning,
            Self::Kafka=>t.success,Self::Nginx=>t.secondary,Self::Tomcat=>t.primary,
            Self::Caddy=>t.accent,Self::Mysql=>Color::Rgb(0,120,215),
            Self::Postgresql=>Color::Rgb(51,103,145),Self::Mongodb=>Color::Rgb(77,179,61),
            Self::Rabbitmq=>Color::Rgb(255,102,0),Self::Haproxy=>Color::Rgb(99,179,71),
            Self::Keepalived=>Color::Rgb(175,41,41),Self::Etcd=>Color::Rgb(65,154,222),
            Self::Consul=>Color::Rgb(220,55,125),Self::Zookeeper=>Color::Rgb(218,149,50),
            Self::Minio=>Color::Rgb(200,46,46),Self::Prometheus=>Color::Rgb(218,78,41),
            Self::Grafana=>Color::Rgb(244,104,50),Self::Memcached=>Color::Rgb(138,180,248),Self::Docker=>Color::Rgb(13,183,237),
        }
    }
    pub fn all() -> Vec<MiddlewareKind> {
        vec![Self::Overview,Self::Redis,Self::Mysql,Self::Postgresql,
            Self::Mongodb,Self::Elasticsearch,Self::Kafka,Self::Rabbitmq,
            Self::Nginx,Self::Tomcat,Self::Caddy,Self::Haproxy,Self::Keepalived,
            Self::Etcd,Self::Consul,Self::Zookeeper,Self::Minio,Self::Prometheus,
            Self::Grafana,Self::Memcached,Self::Docker]
    }
    pub fn manageable() -> Vec<MiddlewareKind> {
        vec![Self::Redis,Self::Elasticsearch,Self::Kafka,Self::Nginx,Self::Tomcat,
            Self::Caddy,Self::Mysql,Self::Postgresql,Self::Mongodb,Self::Rabbitmq,
            Self::Haproxy,Self::Keepalived,Self::Docker]
    }
    pub fn from_discovery(d:&DiscoveredService) -> MiddlewareKind {
        match d.mw_type.as_str() {
            "redis"=>Self::Redis,"mysql"=>Self::Mysql,"postgresql"=>Self::Postgresql,
            "mongodb"=>Self::Mongodb,"nginx"=>Self::Nginx,"apache"=>Self::Nginx,
            "tomcat"=>Self::Tomcat,"caddy"=>Self::Caddy,"kafka"=>Self::Kafka,
            "elasticsearch"=>Self::Elasticsearch,"rabbitmq"=>Self::Rabbitmq,
            "haproxy"=>Self::Haproxy,"keepalived"=>Self::Keepalived,
            "etcd"=>Self::Etcd,"consul"=>Self::Consul,"zookeeper"=>Self::Zookeeper,
            "minio"=>Self::Minio,"prometheus"=>Self::Prometheus,"grafana"=>Self::Grafana,
            "memcached"=>Self::Memcached,"docker"=>Self::Docker,_=>Self::Overview,
        }
    }
}
