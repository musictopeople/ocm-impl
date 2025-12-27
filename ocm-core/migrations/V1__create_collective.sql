CREATE TABLE IF NOT EXISTS individual(
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    first_name TEXT NOT NULL,
    middle_name TEXT,
    last_name TEXT,
    dob TEXT, -- ISO 8601 format: YYYY-MM-DD HH:MM:SS
    phone TEXT,
    email TEXT,
    employer TEXT,
    updated_on TEXT DEFAULT (datetime('now')),
    UNIQUE(first_name,middle_name,last_name,dob)
);

CREATE TABLE IF NOT EXISTS location(
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    email TEXT,
    phone TEXT,
    address TEXT,
    city TEXT,
    state TEXT,
    zip TEXT,
    country TEXT,
    coordinates_lat REAL,
    coordinates_lon REAL,
    updated_on TEXT DEFAULT (datetime('now')),
    UNIQUE(address,city,state,zip,country)
);

CREATE TABLE IF NOT EXISTS affiliation(
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    name TEXT NOT NULL,
    affiliation_type TEXT NOT NULL CHECK(affiliation_type IN ('RANGE', 'VALUE', 'COHORT')),
    value TEXT,
    range_min INTEGER,
    range_max INTEGER,
    cohort TEXT,
    updated_on TEXT DEFAULT (datetime('now')),
    UNIQUE(affiliation_type,value,range_min,range_max,cohort)
);

CREATE TABLE IF NOT EXISTS condition(
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    name TEXT NOT NULL,
    condition_type TEXT NOT NULL CHECK(condition_type IN ('AGE', 'COORDINATES')),
    age_min INTEGER,
    age_max INTEGER,
    calculated_age_from TEXT,
    calculated_age_to TEXT,
    coordinates_lat REAL,
    coordinates_lon REAL,
    distance REAL,
    updated_on TEXT DEFAULT (datetime('now')),
    UNIQUE(condition_type,age_min,age_max,calculated_age_from,calculated_age_to,coordinates_lat,coordinates_lon,distance)
);

CREATE TABLE IF NOT EXISTS cohort(
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    name TEXT NOT NULL,
    capacity REAL,
    updated_on TEXT DEFAULT (datetime('now')),
    UNIQUE(name)
);

CREATE TABLE IF NOT EXISTS experience(
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    name TEXT NOT NULL,
    updated_on TEXT DEFAULT (datetime('now')),
    UNIQUE(name)
);

CREATE TABLE IF NOT EXISTS schedule(
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    "from" TEXT,
    "to" TEXT,
    days_of_week_min INTEGER,
    days_of_week_max INTEGER,
    UNIQUE("from", "to", days_of_week_min, days_of_week_max)
);

-- Junction and relationship tables

CREATE TABLE IF NOT EXISTS individual_location(
    individual_id TEXT NOT NULL,
    location_id TEXT NOT NULL,
    PRIMARY KEY (individual_id, location_id),
    FOREIGN KEY (individual_id) REFERENCES individual(id) ON DELETE CASCADE,
    FOREIGN KEY (location_id) REFERENCES location(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS experience_location(
    experience_id TEXT NOT NULL,
    location_id TEXT NOT NULL,
    PRIMARY KEY (experience_id, location_id),
    FOREIGN KEY (experience_id) REFERENCES experience(id) ON DELETE CASCADE,
    FOREIGN KEY (location_id) REFERENCES location(id) ON DELETE CASCADE
);


CREATE TABLE IF NOT EXISTS individual_affiliation(
    individual_id TEXT NOT NULL,
    affiliation_id TEXT NOT NULL,
    updated_on TEXT DEFAULT (datetime('now')),
    PRIMARY KEY (individual_id, affiliation_id),
    FOREIGN KEY (individual_id) REFERENCES individual(id) ON DELETE CASCADE,
    FOREIGN KEY (affiliation_id) REFERENCES affiliation(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS facilitator_affiliation(
    individual_id TEXT NOT NULL,
    affiliation_id TEXT NOT NULL,
    updated_on TEXT DEFAULT (datetime('now')),
    PRIMARY KEY (individual_id, affiliation_id),
    FOREIGN KEY (individual_id) REFERENCES individual(id) ON DELETE CASCADE,
    FOREIGN KEY (affiliation_id) REFERENCES affiliation(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS cohort_affiliation(
    cohort_id TEXT NOT NULL,
    affiliation_id TEXT NOT NULL,
    updated_on TEXT DEFAULT (datetime('now')),
    PRIMARY KEY(cohort_id, affiliation_id),
    FOREIGN KEY (cohort_id) REFERENCES cohort(id) ON DELETE CASCADE,
    FOREIGN KEY (affiliation_id) REFERENCES affiliation(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS cohort_capacity(
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    count REAL NOT NULL,
    threshold REAL NOT NULL,
    updated_on TEXT DEFAULT (datetime('now')),
    FOREIGN KEY (id) REFERENCES cohort(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS experience_cohort(
    experience_id TEXT NOT NULL,
    cohort_id TEXT NOT NULL,
    updated_on TEXT DEFAULT (datetime('now')),
    PRIMARY KEY (experience_id, cohort_id),
    FOREIGN KEY (experience_id) REFERENCES experience(id) ON DELETE CASCADE,
    FOREIGN KEY (cohort_id) REFERENCES cohort(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS experience_sign_in(
    individual_id TEXT NOT NULL,
    experience_id TEXT NOT NULL,
    updated_on TEXT DEFAULT (datetime('now')),
    PRIMARY KEY (individual_id, experience_id, updated_on),
    FOREIGN KEY (individual_id) REFERENCES individual(id) ON DELETE CASCADE,
    FOREIGN KEY (experience_id) REFERENCES experience(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS experience_sign_out(
    individual_id TEXT NOT NULL,
    experience_id TEXT NOT NULL,
    updated_on TEXT DEFAULT (datetime('now')),
    PRIMARY KEY (individual_id, experience_id, updated_on),
    FOREIGN KEY (individual_id) REFERENCES individual(id) ON DELETE CASCADE,
    FOREIGN KEY (experience_id) REFERENCES experience(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS collective(
    individual_id TEXT,
    experience_id TEXT,
    cohort_id TEXT,
    updated_on TEXT DEFAULT (datetime('now')),
    PRIMARY KEY(individual_id, experience_id, cohort_id),
    FOREIGN KEY (individual_id) REFERENCES individual(id) ON DELETE CASCADE,
    FOREIGN KEY (experience_id) REFERENCES experience(id) ON DELETE CASCADE,
    FOREIGN KEY (cohort_id) REFERENCES cohort(id) ON DELETE CASCADE
);