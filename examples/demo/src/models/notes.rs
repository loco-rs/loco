use sea_orm::{entity::prelude::*, Set};
use serde::{Deserialize, Serialize};

use super::_entities::notes::ActiveModel;

impl ActiveModelBehavior for ActiveModel {
    // extend activemodel below (keep comment for generators)
}
