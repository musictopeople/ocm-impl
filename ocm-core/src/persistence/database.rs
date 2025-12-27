use crate::core::error::{OcmError, Result};
use crate::core::models::*;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path).map_err(OcmError::Database)?;
        Ok(Database {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn get_connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|_| OcmError::Database(rusqlite::Error::InvalidPath("Mutex poisoned".into())))
    }

    pub fn create_individual(&self, individual: &Individual) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            Individual::insert_sql(),
            (
                &individual.id,
                &individual.first_name,
                &individual.middle_name,
                &individual.last_name,
                &individual.dob,
                &individual.phone,
                &individual.email,
                &individual.employer,
                &individual.updated_on,
            ),
        )?;
        Ok(())
    }

    pub fn get<T: DatabaseModel>(&self, id: &str) -> Result<Option<T>> {
        let sql = format!(
            "SELECT {} FROM {} WHERE id = ?1",
            T::select_fields(),
            T::table_name()
        );
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(&sql)?;

        let mut rows = stmt.query_map([id], |row| T::from_row(row))?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn update_individual(&self, individual: &Individual) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            Individual::update_sql(),
            (
                &individual.id,
                &individual.first_name,
                &individual.middle_name,
                &individual.last_name,
                &individual.dob,
                &individual.phone,
                &individual.email,
                &individual.employer,
                &individual.updated_on,
            ),
        )?;
        Ok(())
    }

    pub fn delete<T: DatabaseModel>(&self, id: &str) -> Result<()> {
        let sql = format!("DELETE FROM {} WHERE id = ?1", T::table_name());
        let conn = self.get_connection()?;
        conn.execute(&sql, [id])?;
        Ok(())
    }

    pub fn list<T: DatabaseModel>(&self) -> Result<Vec<T>> {
        let sql = format!("SELECT {} FROM {}", T::select_fields(), T::table_name());
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(&sql)?;

        let rows = stmt.query_map([], |row| T::from_row(row))?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn get_individual(&self, id: &str) -> Result<Option<Individual>> {
        let sql = format!(
            "SELECT {} FROM {} WHERE id = ?1",
            Individual::select_fields(),
            Individual::table_name()
        );
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(&sql)?;

        let mut rows = stmt.query_map([id], |row| Individual::from_row(row))?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn delete_individual(&self, id: &str) -> Result<()> {
        self.delete::<Individual>(id)
    }

    pub fn list_individuals(&self) -> Result<Vec<Individual>> {
        self.list()
    }

    // SignedMemory CRUD operations
    pub fn create_signed_memory(&self, memory: &SignedMemory) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            SignedMemory::insert_sql(),
            (
                &memory.id,
                &memory.did,
                &memory.memory_type,
                &memory.memory_data,
                &memory.content_hash,
                &memory.signature,
                &memory.timestamp,
                &memory.updated_on,
            ),
        )?;
        Ok(())
    }

    pub fn get_signed_memory(&self, id: &str) -> Result<Option<SignedMemory>> {
        self.get(id)
    }

    pub fn list_signed_memories(&self) -> Result<Vec<SignedMemory>> {
        self.list()
    }

    pub fn list_memories_by_did(&self, did: &str) -> Result<Vec<SignedMemory>> {
        let sql = format!(
            "SELECT {} FROM {} WHERE did = ?1 ORDER BY timestamp DESC",
            SignedMemory::select_fields(),
            SignedMemory::table_name()
        );
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(&sql)?;

        let rows = stmt.query_map([did], |row| SignedMemory::from_row(row))?;
        let mut memories = Vec::new();
        for row in rows {
            memories.push(row?);
        }
        Ok(memories)
    }

    // Location CRUD operations
    pub fn create_location(&self, location: &Location) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT INTO location (id, email, phone, address, city, state, zip, country, coordinates_lat, coordinates_lon, updated_on)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            (
                &location.id,
                &location.email,
                &location.phone,
                &location.address,
                &location.city,
                &location.state,
                &location.zip,
                &location.country,
                &location.coordinates_lat,
                &location.coordinates_lon,
                &location.updated_on,
            ),
        )?;
        Ok(())
    }

    pub fn get_location(&self, id: &str) -> Result<Option<Location>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, email, phone, address, city, state, zip, country, coordinates_lat, coordinates_lon, updated_on 
             FROM location WHERE id = ?1"
        )?;

        let mut rows = stmt.query_map([id], |row| {
            Ok(Location {
                id: row.get(0)?,
                email: row.get(1)?,
                phone: row.get(2)?,
                address: row.get(3)?,
                city: row.get(4)?,
                state: row.get(5)?,
                zip: row.get(6)?,
                country: row.get(7)?,
                coordinates_lat: row.get(8)?,
                coordinates_lon: row.get(9)?,
                updated_on: row.get(10)?,
            })
        })?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn update_location(&self, location: &Location) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "UPDATE location SET email = ?2, phone = ?3, address = ?4, city = ?5, state = ?6, 
             zip = ?7, country = ?8, coordinates_lat = ?9, coordinates_lon = ?10, updated_on = ?11 
             WHERE id = ?1",
            (
                &location.id,
                &location.email,
                &location.phone,
                &location.address,
                &location.city,
                &location.state,
                &location.zip,
                &location.country,
                &location.coordinates_lat,
                &location.coordinates_lon,
                &location.updated_on,
            ),
        )?;
        Ok(())
    }

    pub fn delete_location(&self, id: &str) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute("DELETE FROM location WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn list_locations(&self) -> Result<Vec<Location>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, email, phone, address, city, state, zip, country, coordinates_lat, coordinates_lon, updated_on 
             FROM location"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Location {
                id: row.get(0)?,
                email: row.get(1)?,
                phone: row.get(2)?,
                address: row.get(3)?,
                city: row.get(4)?,
                state: row.get(5)?,
                zip: row.get(6)?,
                country: row.get(7)?,
                coordinates_lat: row.get(8)?,
                coordinates_lon: row.get(9)?,
                updated_on: row.get(10)?,
            })
        })?;

        let mut locations = Vec::new();
        for row in rows {
            locations.push(row?);
        }
        Ok(locations)
    }

    // Experience CRUD operations
    pub fn create_experience(&self, experience: &Experience) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT INTO experience (id, name, updated_on) VALUES (?1, ?2, ?3)",
            (&experience.id, &experience.name, &experience.updated_on),
        )?;
        Ok(())
    }

    pub fn get_experience(&self, id: &str) -> Result<Option<Experience>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT id, name, updated_on FROM experience WHERE id = ?1")?;
        let mut rows = stmt.query_map([id], |row| {
            Ok(Experience {
                id: row.get(0)?,
                name: row.get(1)?,
                updated_on: row.get(2)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn update_experience(&self, experience: &Experience) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "UPDATE experience SET name = ?2, updated_on = ?3 WHERE id = ?1",
            (&experience.id, &experience.name, &experience.updated_on),
        )?;
        Ok(())
    }

    pub fn delete_experience(&self, id: &str) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute("DELETE FROM experience WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn list_experiences(&self) -> Result<Vec<Experience>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT id, name, updated_on FROM experience")?;
        let rows = stmt.query_map([], |row| {
            Ok(Experience {
                id: row.get(0)?,
                name: row.get(1)?,
                updated_on: row.get(2)?,
            })
        })?;
        let mut experiences = Vec::new();
        for row in rows {
            experiences.push(row?);
        }
        Ok(experiences)
    }

    // Cohort CRUD operations
    pub fn create_cohort(&self, cohort: &Cohort) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT INTO cohort (id, name, capacity, updated_on) VALUES (?1, ?2, ?3, ?4)",
            (
                &cohort.id,
                &cohort.name,
                &cohort.capacity,
                &cohort.updated_on,
            ),
        )?;
        Ok(())
    }

    pub fn get_cohort(&self, id: &str) -> Result<Option<Cohort>> {
        let conn = self.get_connection()?;
        let mut stmt =
            conn.prepare("SELECT id, name, capacity, updated_on FROM cohort WHERE id = ?1")?;
        let mut rows = stmt.query_map([id], |row| {
            Ok(Cohort {
                id: row.get(0)?,
                name: row.get(1)?,
                capacity: row.get(2)?,
                updated_on: row.get(3)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn update_cohort(&self, cohort: &Cohort) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "UPDATE cohort SET name = ?2, capacity = ?3, updated_on = ?4 WHERE id = ?1",
            (
                &cohort.id,
                &cohort.name,
                &cohort.capacity,
                &cohort.updated_on,
            ),
        )?;
        Ok(())
    }

    pub fn delete_cohort(&self, id: &str) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute("DELETE FROM cohort WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn list_cohorts(&self) -> Result<Vec<Cohort>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT id, name, capacity, updated_on FROM cohort")?;
        let rows = stmt.query_map([], |row| {
            Ok(Cohort {
                id: row.get(0)?,
                name: row.get(1)?,
                capacity: row.get(2)?,
                updated_on: row.get(3)?,
            })
        })?;
        let mut cohorts = Vec::new();
        for row in rows {
            cohorts.push(row?);
        }
        Ok(cohorts)
    }

    // Schedule CRUD operations
    pub fn create_schedule(&self, schedule: &Schedule) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT INTO schedule (id, \"from\", \"to\", days_of_week_min, days_of_week_max) VALUES (?1, ?2, ?3, ?4, ?5)",
            (&schedule.id, &schedule.from, &schedule.to, &schedule.days_of_week_min, &schedule.days_of_week_max),
        )?;
        Ok(())
    }

    pub fn get_schedule(&self, id: &str) -> Result<Option<Schedule>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT id, \"from\", \"to\", days_of_week_min, days_of_week_max FROM schedule WHERE id = ?1")?;
        let mut rows = stmt.query_map([id], |row| {
            Ok(Schedule {
                id: row.get(0)?,
                from: row.get(1)?,
                to: row.get(2)?,
                days_of_week_min: row.get(3)?,
                days_of_week_max: row.get(4)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn update_schedule(&self, schedule: &Schedule) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "UPDATE schedule SET \"from\" = ?2, \"to\" = ?3, days_of_week_min = ?4, days_of_week_max = ?5 WHERE id = ?1",
            (&schedule.id, &schedule.from, &schedule.to, &schedule.days_of_week_min, &schedule.days_of_week_max),
        )?;
        Ok(())
    }

    pub fn delete_schedule(&self, id: &str) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute("DELETE FROM schedule WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn list_schedules(&self) -> Result<Vec<Schedule>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, \"from\", \"to\", days_of_week_min, days_of_week_max FROM schedule",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Schedule {
                id: row.get(0)?,
                from: row.get(1)?,
                to: row.get(2)?,
                days_of_week_min: row.get(3)?,
                days_of_week_max: row.get(4)?,
            })
        })?;
        let mut schedules = Vec::new();
        for row in rows {
            schedules.push(row?);
        }
        Ok(schedules)
    }

    // Affiliation CRUD operations
    pub fn create_affiliation(&self, affiliation: &Affiliation) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT INTO affiliation (id, name, affiliation_type, value, range_min, range_max, cohort, updated_on)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            (
                &affiliation.id,
                &affiliation.name,
                &affiliation.affiliation_type.to_string(),
                &affiliation.value,
                &affiliation.range_min,
                &affiliation.range_max,
                &affiliation.cohort,
                &affiliation.updated_on,
            ),
        )?;
        Ok(())
    }

    pub fn get_affiliation(&self, id: &str) -> Result<Option<Affiliation>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, affiliation_type, value, range_min, range_max, cohort, updated_on 
             FROM affiliation WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map([id], |row| {
            let affiliation_type_str: String = row.get(2)?;
            let affiliation_type =
                AffiliationType::from_string(&affiliation_type_str).map_err(|e| {
                    rusqlite::Error::InvalidColumnType(2, e.into(), rusqlite::types::Type::Text)
                })?;

            Ok(Affiliation {
                id: row.get(0)?,
                name: row.get(1)?,
                affiliation_type,
                value: row.get(3)?,
                range_min: row.get(4)?,
                range_max: row.get(5)?,
                cohort: row.get(6)?,
                updated_on: row.get(7)?,
            })
        })?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn update_affiliation(&self, affiliation: &Affiliation) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "UPDATE affiliation SET name = ?2, affiliation_type = ?3, value = ?4, range_min = ?5, 
             range_max = ?6, cohort = ?7, updated_on = ?8 WHERE id = ?1",
            (
                &affiliation.id,
                &affiliation.name,
                &affiliation.affiliation_type.to_string(),
                &affiliation.value,
                &affiliation.range_min,
                &affiliation.range_max,
                &affiliation.cohort,
                &affiliation.updated_on,
            ),
        )?;
        Ok(())
    }

    pub fn delete_affiliation(&self, id: &str) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute("DELETE FROM affiliation WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn list_affiliations(&self) -> Result<Vec<Affiliation>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, affiliation_type, value, range_min, range_max, cohort, updated_on 
             FROM affiliation",
        )?;

        let rows = stmt.query_map([], |row| {
            let affiliation_type_str: String = row.get(2)?;
            let affiliation_type =
                AffiliationType::from_string(&affiliation_type_str).map_err(|e| {
                    rusqlite::Error::InvalidColumnType(2, e.into(), rusqlite::types::Type::Text)
                })?;

            Ok(Affiliation {
                id: row.get(0)?,
                name: row.get(1)?,
                affiliation_type,
                value: row.get(3)?,
                range_min: row.get(4)?,
                range_max: row.get(5)?,
                cohort: row.get(6)?,
                updated_on: row.get(7)?,
            })
        })?;

        let mut affiliations = Vec::new();
        for row in rows {
            affiliations.push(row?);
        }
        Ok(affiliations)
    }

    // Condition CRUD operations
    pub fn create_condition(&self, condition: &Condition) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT INTO condition (id, name, condition_type, age_min, age_max, calculated_age_from, calculated_age_to, coordinates_lat, coordinates_lon, distance, updated_on)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            (
                &condition.id,
                &condition.name,
                &condition.condition_type.to_string(),
                &condition.age_min,
                &condition.age_max,
                &condition.calculated_age_from,
                &condition.calculated_age_to,
                &condition.coordinates_lat,
                &condition.coordinates_lon,
                &condition.distance,
                &condition.updated_on,
            ),
        )?;
        Ok(())
    }

    pub fn get_condition(&self, id: &str) -> Result<Option<Condition>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, condition_type, age_min, age_max, calculated_age_from, calculated_age_to, coordinates_lat, coordinates_lon, distance, updated_on 
             FROM condition WHERE id = ?1"
        )?;

        let mut rows = stmt.query_map([id], |row| {
            let condition_type_str: String = row.get(2)?;
            let condition_type = ConditionType::from_string(&condition_type_str).map_err(|e| {
                rusqlite::Error::InvalidColumnType(2, e.into(), rusqlite::types::Type::Text)
            })?;

            Ok(Condition {
                id: row.get(0)?,
                name: row.get(1)?,
                condition_type,
                age_min: row.get(3)?,
                age_max: row.get(4)?,
                calculated_age_from: row.get(5)?,
                calculated_age_to: row.get(6)?,
                coordinates_lat: row.get(7)?,
                coordinates_lon: row.get(8)?,
                distance: row.get(9)?,
                updated_on: row.get(10)?,
            })
        })?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn update_condition(&self, condition: &Condition) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "UPDATE condition SET name = ?2, condition_type = ?3, age_min = ?4, age_max = ?5, 
             calculated_age_from = ?6, calculated_age_to = ?7, coordinates_lat = ?8, coordinates_lon = ?9, 
             distance = ?10, updated_on = ?11 WHERE id = ?1",
            (
                &condition.id,
                &condition.name,
                &condition.condition_type.to_string(),
                &condition.age_min,
                &condition.age_max,
                &condition.calculated_age_from,
                &condition.calculated_age_to,
                &condition.coordinates_lat,
                &condition.coordinates_lon,
                &condition.distance,
                &condition.updated_on,
            ),
        )?;
        Ok(())
    }

    pub fn delete_condition(&self, id: &str) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute("DELETE FROM condition WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn list_conditions(&self) -> Result<Vec<Condition>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, condition_type, age_min, age_max, calculated_age_from, calculated_age_to, coordinates_lat, coordinates_lon, distance, updated_on 
             FROM condition"
        )?;

        let rows = stmt.query_map([], |row| {
            let condition_type_str: String = row.get(2)?;
            let condition_type = ConditionType::from_string(&condition_type_str).map_err(|e| {
                rusqlite::Error::InvalidColumnType(2, e.into(), rusqlite::types::Type::Text)
            })?;

            Ok(Condition {
                id: row.get(0)?,
                name: row.get(1)?,
                condition_type,
                age_min: row.get(3)?,
                age_max: row.get(4)?,
                calculated_age_from: row.get(5)?,
                calculated_age_to: row.get(6)?,
                coordinates_lat: row.get(7)?,
                coordinates_lon: row.get(8)?,
                distance: row.get(9)?,
                updated_on: row.get(10)?,
            })
        })?;

        let mut conditions = Vec::new();
        for row in rows {
            conditions.push(row?);
        }
        Ok(conditions)
    }

    // Claim Token CRUD operations
    pub fn create_claim_token(&self, token: &ClaimToken) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            ClaimToken::insert_sql(),
            (
                &token.id,
                &token.token,
                &token.memory_id,
                &token.organization_did,
                &token.expiry_timestamp,
                &token.claimed_by_did,
                &token.claimed_timestamp,
                &token.created_timestamp,
                &token.updated_on,
            ),
        )?;
        Ok(())
    }

    pub fn get_claim_token(&self, id: &str) -> Result<Option<ClaimToken>> {
        let sql = format!(
            "SELECT {} FROM {} WHERE id = ?1",
            ClaimToken::select_fields(),
            ClaimToken::table_name()
        );
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query_map([id], ClaimToken::from_row)?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn get_claim_token_by_token(&self, token: &str) -> Result<Option<ClaimToken>> {
        let sql = format!(
            "SELECT {} FROM {} WHERE token = ?1",
            ClaimToken::select_fields(),
            ClaimToken::table_name()
        );
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query_map([token], ClaimToken::from_row)?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn update_claim_token(&self, token: &ClaimToken) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            ClaimToken::update_sql(),
            (
                &token.id,
                &token.token,
                &token.memory_id,
                &token.organization_did,
                &token.expiry_timestamp,
                &token.claimed_by_did,
                &token.claimed_timestamp,
                &token.created_timestamp,
                &token.updated_on,
            ),
        )?;
        Ok(())
    }

    pub fn list_claim_tokens_by_organization(
        &self,
        organization_did: &str,
    ) -> Result<Vec<ClaimToken>> {
        let sql = format!(
            "SELECT {} FROM {} WHERE organization_did = ?1 ORDER BY created_timestamp DESC",
            ClaimToken::select_fields(),
            ClaimToken::table_name()
        );
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([organization_did], ClaimToken::from_row)?;

        let mut tokens = Vec::new();
        for row in rows {
            tokens.push(row?);
        }
        Ok(tokens)
    }

    // Proxy Memory CRUD operations
    pub fn create_proxy_memory(&self, proxy: &ProxyMemory) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            ProxyMemory::insert_sql(),
            (
                &proxy.id,
                &proxy.proxy_for_name,
                &proxy.proxy_for_info,
                &proxy.organization_did,
                &proxy.memory_data,
                &proxy.created_timestamp,
                &proxy.claim_token_id,
            ),
        )?;
        Ok(())
    }

    pub fn get_proxy_memory(&self, id: &str) -> Result<Option<ProxyMemory>> {
        let sql = format!(
            "SELECT {} FROM {} WHERE id = ?1",
            ProxyMemory::select_fields(),
            ProxyMemory::table_name()
        );
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query_map([id], ProxyMemory::from_row)?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn list_proxy_memories_by_organization(
        &self,
        organization_did: &str,
    ) -> Result<Vec<ProxyMemory>> {
        let sql = format!(
            "SELECT {} FROM {} WHERE organization_did = ?1 ORDER BY created_timestamp DESC",
            ProxyMemory::select_fields(),
            ProxyMemory::table_name()
        );
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([organization_did], ProxyMemory::from_row)?;

        let mut proxies = Vec::new();
        for row in rows {
            proxies.push(row?);
        }
        Ok(proxies)
    }

    pub fn search_proxy_memories_by_name(&self, name_pattern: &str) -> Result<Vec<ProxyMemory>> {
        let sql = format!(
            "SELECT {} FROM {} WHERE proxy_for_name LIKE ?1 ORDER BY created_timestamp DESC",
            ProxyMemory::select_fields(),
            ProxyMemory::table_name()
        );
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(&sql)?;

        // Escape SQL wildcards to prevent injection
        let escaped_pattern = name_pattern
            .replace('\\', "\\\\") // Escape backslashes first
            .replace('%', "\\%") // Escape percent signs
            .replace('_', "\\_"); // Escape underscores
        let search_pattern = format!("%{}%", escaped_pattern);

        let rows = stmt.query_map([search_pattern], ProxyMemory::from_row)?;

        let mut proxies = Vec::new();
        for row in rows {
            proxies.push(row?);
        }
        Ok(proxies)
    }
}
