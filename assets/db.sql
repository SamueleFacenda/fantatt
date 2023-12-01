CREATE TABLE player (
	id INTEGER NOT NULL, 
	name VARCHAR(80) NOT NULL, 
	score INTEGER, 
	PRIMARY KEY (id)
);


CREATE UNIQUE INDEX ix_player_name ON player (name);

CREATE TABLE event (
	id INTEGER NOT NULL, 
	name VARCHAR(80) NOT NULL, 
	date DATETIME, 
	PRIMARY KEY (id)
);


CREATE UNIQUE INDEX ix_event_name ON event (name);

CREATE TABLE "match" (
	id INTEGER NOT NULL, 
	one_id INTEGER NOT NULL, 
	two_id INTEGER NOT NULL, 
	_score VARCHAR(80) NOT NULL, 
	event_id INTEGER NOT NULL, 
	PRIMARY KEY (id), 
	FOREIGN KEY(one_id) REFERENCES player (id), 
	FOREIGN KEY(two_id) REFERENCES player (id), 
	FOREIGN KEY(event_id) REFERENCES event (id)
);
