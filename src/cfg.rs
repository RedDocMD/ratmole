use cargo_platform::{Cfg as CargoCfg, CfgExpr as CargoCfgExpr, Platform as CargoPlatform};

pub enum CfgExpr {
    Not(Box<CfgExpr>),
    All(Vec<CfgExpr>),
    Any(Vec<CfgExpr>),
    Value(Cfg),
}

#[derive(PartialEq, Eq)]
pub enum Cfg {
    Value(String),
    KeyValue(String, String),
}

pub enum Platform {
    Name(String),
    CfgExpr(CfgExpr),
}

impl From<CargoCfg> for Cfg {
    fn from(cfg: CargoCfg) -> Self {
        match cfg {
            CargoCfg::Name(name) => Cfg::Value(name),
            CargoCfg::KeyPair(key, value) => Cfg::KeyValue(key, value),
        }
    }
}

impl From<&str> for Cfg {
    fn from(value: &str) -> Self {
        Cfg::Value(String::from(value))
    }
}

impl From<(&str, &str)> for Cfg {
    fn from((key, value): (&str, &str)) -> Self {
        Cfg::KeyValue(String::from(key), String::from(value))
    }
}

impl From<CargoCfgExpr> for CfgExpr {
    fn from(cfg_expr: CargoCfgExpr) -> Self {
        match cfg_expr {
            CargoCfgExpr::Not(cfg_expr) => CfgExpr::Not(Box::new((*cfg_expr).into())),
            CargoCfgExpr::All(cfg_exprs) => CfgExpr::All(
                cfg_exprs
                    .into_iter()
                    .map(|cfg_expr| cfg_expr.into())
                    .collect(),
            ),
            CargoCfgExpr::Any(cfg_exprs) => CfgExpr::Any(
                cfg_exprs
                    .into_iter()
                    .map(|cfg_expr| cfg_expr.into())
                    .collect(),
            ),
            CargoCfgExpr::Value(cfg) => CfgExpr::Value(cfg.into()),
        }
    }
}

impl From<CargoPlatform> for Platform {
    fn from(platform: CargoPlatform) -> Self {
        match platform {
            CargoPlatform::Name(name) => Platform::Name(name),
            CargoPlatform::Cfg(cfg_expr) => Platform::CfgExpr(cfg_expr.into()),
        }
    }
}

impl CfgExpr {
    fn is_satisfied_by_slice(&self, cfg: &[&Cfg]) -> bool {
        match self {
            CfgExpr::Not(e) => !e.is_satisfied_by_slice(cfg),
            CfgExpr::All(e) => e.iter().all(|x| x.is_satisfied_by_slice(cfg)),
            CfgExpr::Any(e) => e.iter().any(|x| x.is_satisfied_by_slice(cfg)),
            CfgExpr::Value(e) => cfg.contains(&e),
        }
    }

    pub fn is_satisfied_by(&self, ot: &Self) -> bool {
        match ot {
            CfgExpr::Not(e) => !self.is_satisfied_by(e),
            CfgExpr::All(e) => e.iter().all(|x| self.is_satisfied_by(x)),
            CfgExpr::Any(e) => e.iter().any(|x| self.is_satisfied_by(x)),
            CfgExpr::Value(e) => self.is_satisfied_by_slice(&[e]),
        }
    }
}

pub(crate) fn dev_cfg_expr() -> CfgExpr {
    CfgExpr::All(vec![
        CfgExpr::Value(Cfg::from(("target_arch", "x86_64"))),
        CfgExpr::Value(Cfg::from(("target_feature", "fxsr"))),
        CfgExpr::Value(Cfg::from(("target_feature", "sse"))),
        CfgExpr::Value(Cfg::from(("target_feature", "sse2"))),
        CfgExpr::Value(Cfg::from(("target_os", "linux"))),
        CfgExpr::Value(Cfg::from(("target_family", "unix"))),
        CfgExpr::Value(Cfg::from(("target_env", "gnu"))),
        CfgExpr::Value(Cfg::from(("target_endian", "little"))),
        CfgExpr::Value(Cfg::from(("target_pointer_width", "64"))),
        CfgExpr::Value(Cfg::from(("target_vendor", "unknown"))),
    ])
}

pub(crate) fn dev_platform_name() -> String {
    String::from("x86_64-unknown-linux-gnu")
}
