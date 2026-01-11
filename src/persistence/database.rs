use rusqlite::{Connection, Result, params};
use big_space::GridCell;
use std::sync::{Arc, Mutex};
use bevy::prelude::*;
use crate::universe::{DetailedPlanet, PlanetType, SectorIndex, StarDetails};
use serde::{Serialize, Deserialize};
use serde_json;

#[derive(Serialize, Deserialize)]
struct SavedStar {
    x: i64,
    y: i64,
    z: i64,
    details: StarDetails,
}

#[derive(Debug, Clone)]
pub struct DiscoveredWorld {
    pub cell_x: i64,
    pub cell_y: i64,
    pub cell_z: i64,
    pub name: String,
    pub finder: String,
    pub note: String,
    pub date: String,
    pub object_type: String, // "Star" or "Planet"
}

// ... (Existing DiscoveredWorld and imports)
use bevy::prelude::*; // Ensure Vec3 is available

#[derive(Debug, Clone)]
pub struct PlayerState {
    pub cell_x: i64,
    pub cell_y: i64,
    pub cell_z: i64,
    pub local_x: f32,
    pub local_y: f32,
    pub local_z: f32,
    pub vel_x: f32,
    pub vel_y: f32,
    pub vel_z: f32,
    pub timestamp: i64, 
    pub throttle: f32, // Added throttle
}

// Thread-safe wrapper for the connection
#[derive(Resource, Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open() -> Result<Self> {
        let conn = Connection::open("universe.db")?;
        
        // Execute schema creation in a batch
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS discoveries (
                id INTEGER PRIMARY KEY,
                cell_x INTEGER NOT NULL,
                cell_y INTEGER NOT NULL,
                cell_z INTEGER NOT NULL,
                name TEXT NOT NULL,
                finder TEXT NOT NULL,
                note TEXT NOT NULL,
                date TEXT NOT NULL,
                object_type TEXT NOT NULL,
                UNIQUE(cell_x, cell_y, cell_z)
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_discovery_coords ON discoveries (cell_x, cell_y, cell_z);

            CREATE TABLE IF NOT EXISTS sectors (
                x INTEGER NOT NULL,
                y INTEGER NOT NULL,
                z INTEGER NOT NULL,
                data TEXT NOT NULL,
                PRIMARY KEY(x, y, z)
            );
            CREATE INDEX IF NOT EXISTS idx_sector_coords ON sectors (x, y, z);

            CREATE TABLE IF NOT EXISTS player_state (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                cell_x INTEGER NOT NULL,
                cell_y INTEGER NOT NULL,
                cell_z INTEGER NOT NULL,
                local_x REAL NOT NULL,
                local_y REAL NOT NULL,
                local_z REAL NOT NULL,
                vel_x REAL NOT NULL,
                vel_y REAL NOT NULL,
                vel_z REAL NOT NULL,
                timestamp INTEGER NOT NULL,
                throttle REAL DEFAULT 0.0
            );"
        )?;

        // Migration: Add throttle column if it doesn't exist (for existing DBs)
        // We just try to add it and ignore error if it exists
        let _ = conn.execute("ALTER TABLE player_state ADD COLUMN throttle REAL DEFAULT 0.0", []);

        Ok(Database {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
    pub fn seed_predefined_system(&self, scenario: &str) -> Result<()> {
        if scenario != "our_system" {
            return Ok(());
        }

        let sector = SectorIndex { x: 0, y: 0, z: 0 };
        
        // Check if sector is already in DB
        if let Some(_) = self.get_sector_data(sector)? {
            info!("PERSISTENCE: Sector (0,0,0) already seeded. Skipping predefined system.");
            return Ok(());
        }

        info!("PERSISTENCE: Seeding 'Our System' to Sector (0,0,0)...");

        let mut systems = Vec::new();
        let center = GridCell::new(0, 0, 0);

        // 1. Sun + Planets
        let planets = vec![
            DetailedPlanet {
                name: "Mercury".to_string(),
                planet_type: PlanetType::Ice, // Placeholder type for rocky small
                distance: 4000.0,
                size: 10.0,
                color: Color::srgb(0.6, 0.6, 0.6),
                second_color: Some(Color::srgb(0.4, 0.4, 0.4)),
                atmosphere_color: None,
                atmosphere_density: None,
                orbit_speed: 0.047, // Relative orbital speed approx
            },
            DetailedPlanet {
                name: "Venus".to_string(),
                planet_type: PlanetType::Magma,
                distance: 7000.0,
                size: 25.0,
                color: Color::srgb(0.9, 0.8, 0.5),
                second_color: Some(Color::srgb(0.7, 0.6, 0.3)),
                atmosphere_color: Some(Color::srgb(1.0, 0.9, 0.4)),
                atmosphere_density: Some(2.0), // Very thick
                orbit_speed: 0.035,
            },
            DetailedPlanet {
                name: "Earth".to_string(),
                planet_type: PlanetType::Terran,
                distance: 10000.0,
                size: 26.0,
                color: Color::srgb(0.2, 0.5, 0.1), // Green
                second_color: Some(Color::srgb(0.1, 0.3, 0.8)), // Blue Oceans
                atmosphere_color: Some(Color::srgb(0.5, 0.7, 1.0)),
                atmosphere_density: Some(1.0),
                orbit_speed: 0.029,
            },
            DetailedPlanet {
                name: "Mars".to_string(),
                planet_type: PlanetType::Magma, // Rocky Red
                distance: 15000.0,
                size: 14.0,
                color: Color::srgb(0.8, 0.4, 0.2),
                second_color: Some(Color::srgb(0.6, 0.3, 0.1)),
                atmosphere_color: Some(Color::srgb(0.9, 0.5, 0.3)),
                atmosphere_density: Some(0.3), // Thin
                orbit_speed: 0.024,
            },
            DetailedPlanet {
                name: "Jupiter".to_string(),
                planet_type: PlanetType::GasGiant,
                distance: 25000.0,
                size: 80.0,
                color: Color::srgb(0.7, 0.5, 0.3),
                second_color: Some(Color::srgb(0.9, 0.8, 0.6)),
                atmosphere_color: Some(Color::srgb(0.8, 0.6, 0.4)),
                atmosphere_density: Some(1.5),
                orbit_speed: 0.013,
            },
            DetailedPlanet {
                name: "Saturn".to_string(),
                planet_type: PlanetType::GasGiant,
                distance: 35000.0,
                size: 70.0,
                color: Color::srgb(0.8, 0.7, 0.5),
                second_color: Some(Color::srgb(1.0, 0.9, 0.7)),
                atmosphere_color: Some(Color::srgb(0.9, 0.8, 0.6)),
                atmosphere_density: Some(1.3),
                orbit_speed: 0.009,
            },
            DetailedPlanet {
                name: "Uranus".to_string(),
                planet_type: PlanetType::Ice,
                distance: 45000.0,
                size: 40.0,
                color: Color::srgb(0.6, 0.9, 1.0),
                second_color: Some(Color::srgb(0.5, 0.8, 0.9)),
                atmosphere_color: Some(Color::srgb(0.7, 0.9, 1.0)),
                atmosphere_density: Some(0.8),
                orbit_speed: 0.006,
            },
            DetailedPlanet {
                name: "Neptune".to_string(),
                planet_type: PlanetType::Ice,
                distance: 55000.0,
                size: 38.0,
                color: Color::srgb(0.2, 0.3, 1.0),
                second_color: Some(Color::srgb(0.1, 0.2, 0.8)),
                atmosphere_color: Some(Color::srgb(0.3, 0.4, 1.0)),
                atmosphere_density: Some(0.9),
                orbit_speed: 0.005,
            },
        ];

        systems.push((center, StarDetails {
            color: Color::srgb(1.0, 0.9, 0.5), // Sun
            size: 150.0,
            planets: Some(planets),
        }));

        self.save_sector_data(sector, &systems)?;
        Ok(())
    }


    pub fn save_sector_data(&self, sector: SectorIndex, data: &Vec<(GridCell<i64>, StarDetails)>) -> Result<()> {
         let conn = self.conn.lock().unwrap();
         
         // Convert to Serializable format
         let saved_data: Vec<SavedStar> = data.iter().map(|(cell, details)| SavedStar {
             x: cell.x,
             y: cell.y,
             z: cell.z,
             details: details.clone(),
         }).collect();

         // Serialize data
         let json_data = serde_json::to_string(&saved_data).unwrap_or_default();
         
         conn.execute(
             "INSERT OR REPLACE INTO sectors (x, y, z, data) VALUES (?1, ?2, ?3, ?4)",
             params![sector.x, sector.y, sector.z, json_data],
         )?;
         Ok(())
    }

    pub fn get_sector_data(&self, sector: SectorIndex) -> Result<Option<Vec<(GridCell<i64>, StarDetails)>>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT data FROM sectors WHERE x = ?1 AND y = ?2 AND z = ?3")?;
        
        let mut rows = stmt.query(params![sector.x, sector.y, sector.z])?;
        
        if let Some(row) = rows.next()? {
            let json_data: String = row.get(0)?;
            match serde_json::from_str::<Vec<SavedStar>>(&json_data) {
                Ok(saved_data) => {
                    // Convert back
                    let data = saved_data.into_iter().map(|s| (
                        GridCell::new(s.x, s.y, s.z),
                        s.details
                    )).collect();
                    return Ok(Some(data));
                },
                Err(e) => {
                    error!("PERSISTENCE: JSON Deserialization Error for Sector {:?}: {}", sector, e);
                    // Return error to prevent overwriting corrupt data with empty data
                    return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(e))); 
                }
            }
        }
        Ok(None)
    }

    pub fn save_player_state(&self, state: &PlayerState) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO player_state (id, cell_x, cell_y, cell_z, local_x, local_y, local_z, vel_x, vel_y, vel_z, timestamp, throttle)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
             params![state.cell_x, state.cell_y, state.cell_z, 
                     state.local_x, state.local_y, state.local_z, 
                     state.vel_x, state.vel_y, state.vel_z, 
                     state.timestamp, state.throttle],
        )?;
        Ok(())
    }

    pub fn get_player_state(&self) -> Result<Option<PlayerState>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT cell_x, cell_y, cell_z, local_x, local_y, local_z, vel_x, vel_y, vel_z, timestamp, throttle FROM player_state WHERE id = 1")?;
        
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(Some(PlayerState {
                cell_x: row.get(0)?,
                cell_y: row.get(1)?,
                cell_z: row.get(2)?,
                local_x: row.get(3)?,
                local_y: row.get(4)?,
                local_z: row.get(5)?,
                vel_x: row.get(6)?,
                vel_y: row.get(7)?,
                vel_z: row.get(8)?,
                timestamp: row.get(9)?,
                throttle: row.get(10).unwrap_or(0.0), // Handle legacy records if any
            }))
        } else {
            Ok(None)
        }
    }
    // ... (Existing discovery methods)

    pub fn save_discovery(&self, world: &DiscoveredWorld) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO discoveries (cell_x, cell_y, cell_z, name, finder, note, date, object_type)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![world.cell_x, world.cell_y, world.cell_z, world.name, world.finder, world.note, world.date, world.object_type],
        )?;
        Ok(())
    }

    pub fn get_discovery(&self, cell: GridCell<i64>) -> Result<Option<DiscoveredWorld>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT cell_x, cell_y, cell_z, name, finder, note, date, object_type 
             FROM discoveries 
             WHERE cell_x = ?1 AND cell_y = ?2 AND cell_z = ?3"
        )?;
        
        let mut rows = stmt.query(params![cell.x, cell.y, cell.z])?;

        if let Some(row) = rows.next()? {
            Ok(Some(DiscoveredWorld {
                cell_x: row.get(0)?,
                cell_y: row.get(1)?,
                cell_z: row.get(2)?,
                name: row.get(3)?,
                finder: row.get(4)?,
                note: row.get(5)?,
                date: row.get(6)?,
                object_type: row.get(7)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_all_discoveries(&self) -> Result<Vec<DiscoveredWorld>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT cell_x, cell_y, cell_z, name, finder, note, date, object_type FROM discoveries"
        )?;

        let discovery_iter = stmt.query_map([], |row| {
            Ok(DiscoveredWorld {
                cell_x: row.get(0)?,
                cell_y: row.get(1)?,
                cell_z: row.get(2)?,
                name: row.get(3)?,
                finder: row.get(4)?,
                note: row.get(5)?,
                date: row.get(6)?,
                object_type: row.get(7)?,
            })
        })?;

        let mut worlds = Vec::new();
        for world in discovery_iter {
            worlds.push(world?);
        }
        Ok(worlds)
    }
}
