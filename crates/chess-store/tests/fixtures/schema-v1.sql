-- Frozen fixture: a released schema version 1 database with sample rows (SDD 3.11.5, architecture 7.1:
-- one fixture per released schema version). Do not edit: add schema-v2.sql when version 2 ships.
-- The schema part is a copy of migrations/0001_init.sql (checked by tests/open.rs).
-- Schema version 1 of chess.db: SDD-chess-store 3.5.2, equal to architecture 7.2 (see
-- tests/schema_matches_architecture.rs). The constraints are normative.
-- Conventions: every datetime is INTEGER Unix epoch milliseconds in UTC; every bool is INTEGER
-- with CHECK (x IN (0,1)); text identifiers use the default BINARY collation; the connection
-- sets PRAGMA foreign_keys=ON. player.id and game.id are internal row keys (D-40): they never
-- leave chess-store.
CREATE TABLE player (
  id INTEGER PRIMARY KEY,
  uuid TEXT NOT NULL UNIQUE CHECK (uuid = lower(uuid) AND length(uuid) = 36),
  kind TEXT NOT NULL CHECK (kind IN ('human','computer')),
  name TEXT NOT NULL UNIQUE COLLATE BINARY CHECK (length(name) > 0),
  start_category TEXT NOT NULL,
  strict_elo INTEGER NOT NULL,
  training_elo INTEGER NOT NULL,
  strict_rated_games INTEGER NOT NULL DEFAULT 0 CHECK (strict_rated_games >= 0),
  training_rated_games INTEGER NOT NULL DEFAULT 0 CHECK (training_rated_games >= 0),
  reset_at INTEGER,
  archived INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0,1)),
  created_at INTEGER NOT NULL,
  CHECK (kind = 'human' OR reset_at IS NULL)
);
CREATE TABLE computer_profile (
  player_id INTEGER PRIMARY KEY REFERENCES player(id),
  roster_seed INTEGER NOT NULL, roster_index INTEGER NOT NULL,
  skill_level INTEGER NOT NULL, depth_limit INTEGER NOT NULL, movetime_ms INTEGER NOT NULL,
  uci_elo INTEGER,
  error_permille INTEGER NOT NULL CHECK (error_permille BETWEEN 0 AND 1000),
  error_multipv INTEGER NOT NULL,
  CHECK (error_permille = 0 OR error_multipv >= 2),
  UNIQUE (roster_seed, roster_index)
);
-- ENG-2 backstop: no two computers share a parameter set (NULL uci_elo made comparable; every
-- column is an integer, so the index compares integers and never floats).
CREATE UNIQUE INDEX computer_profile_params ON computer_profile
  (skill_level, depth_limit, movetime_ms, COALESCE(uci_elo,-1), error_permille, error_multipv);
CREATE TABLE preferences (
  player_id INTEGER PRIMARY KEY REFERENCES player(id),
  json TEXT NOT NULL CHECK (json_valid(json))
);
CREATE TABLE game (
  id INTEGER PRIMARY KEY,
  uuid TEXT NOT NULL UNIQUE CHECK (uuid = lower(uuid) AND length(uuid) = 36),
  white_id INTEGER REFERENCES player(id), black_id INTEGER REFERENCES player(id),
  matchup TEXT CHECK (matchup IN ('hh','hc','cc')),
  mode TEXT CHECK (mode IN ('training','strict')),
  rated INTEGER NOT NULL CHECK (rated IN (0,1)),
  self_play INTEGER NOT NULL CHECK (self_play IN (0,1)),
  custom_start INTEGER NOT NULL DEFAULT 0 CHECK (custom_start IN (0,1)),
  start_fen TEXT,
  clock_spec TEXT,
  white_clock_ms INTEGER, black_clock_ms INTEGER,
  status TEXT NOT NULL CHECK (status IN ('in_progress','completed','aborted','voided','imported')),
  result TEXT CHECK (result IN ('1-0','0-1','1/2-1/2')),
  termination TEXT, voided_by INTEGER REFERENCES player(id),
  eco TEXT, opening_name TEXT,
  started_at INTEGER, ended_at INTEGER,
  white_time_used_ms INTEGER, black_time_used_ms INTEGER, total_active_ms INTEGER,
  analysis_used INTEGER NOT NULL DEFAULT 0 CHECK (analysis_used IN (0,1)),
  white_elo_before INTEGER, white_elo_after INTEGER, black_elo_before INTEGER, black_elo_after INTEGER,
  content_hash TEXT,
  journal_seq INTEGER NOT NULL DEFAULT 0,
  CHECK ((status = 'in_progress') = (content_hash IS NULL)),
  CHECK (custom_start = (start_fen IS NOT NULL)),
  CHECK (custom_start = 0 OR rated = 0),
  CHECK ((white_clock_ms IS NULL) = (black_clock_ms IS NULL)),
  CHECK (status = 'imported' OR (white_id IS NOT NULL AND black_id IS NOT NULL)),
  CHECK (CASE status WHEN 'completed' THEN result IS NOT NULL
                     WHEN 'imported' THEN 1 ELSE result IS NULL END),
  CHECK (status <> 'imported' OR (matchup IS NULL AND mode IS NULL AND rated = 0)),
  CHECK (status = 'imported' OR (matchup IS NOT NULL AND mode IS NOT NULL)),
  CHECK (mode IS NULL OR mode <> 'training' OR (matchup = 'hc' AND clock_spec IS NULL)),
  CHECK (voided_by IS NULL OR status = 'voided')
);
CREATE INDEX game_white ON game(white_id);
CREATE INDEX game_black ON game(black_id);
CREATE INDEX game_status ON game(status);
CREATE INDEX game_hash ON game(content_hash);
CREATE TABLE move (
  game_id INTEGER NOT NULL REFERENCES game(id), ply INTEGER NOT NULL CHECK (ply >= 1),
  san TEXT NOT NULL, uci TEXT NOT NULL, fen_after TEXT NOT NULL,
  think_ms INTEGER, white_clock_ms INTEGER, black_clock_ms INTEGER, played_at INTEGER,
  PRIMARY KEY (game_id, ply),
  CHECK ((white_clock_ms IS NULL) = (black_clock_ms IS NULL))
) WITHOUT ROWID;
CREATE TABLE move_eval (
  game_id INTEGER NOT NULL, ply INTEGER NOT NULL, cp INTEGER, mate INTEGER,
  -- cp and mate are from WHITE's point of view (decision 12, CR-9.3). cp_loss stays the mover's
  -- point of view (ANA-3).
  best_uci TEXT, cp_loss INTEGER, flag TEXT CHECK (flag IN ('blunder','mistake','inaccuracy')),
  depth INTEGER, engine_version TEXT NOT NULL,
  PRIMARY KEY (game_id, ply), FOREIGN KEY (game_id, ply) REFERENCES move(game_id, ply),
  CHECK (cp IS NULL OR mate IS NULL)
) WITHOUT ROWID;
CREATE TABLE rating_history (
  id INTEGER PRIMARY KEY, player_id INTEGER NOT NULL REFERENCES player(id),
  track TEXT NOT NULL CHECK (track IN ('strict','training')),
  game_id INTEGER REFERENCES game(id),
  rating_before INTEGER NOT NULL, rating_after INTEGER NOT NULL, k INTEGER NOT NULL, at INTEGER NOT NULL
);
CREATE INDEX rating_history_player ON rating_history(player_id, track, id);
-- Decision 13, CR-9.4: one row = one saved line. No player_id (PLY-7 hides a variation with its
-- game). from_ply = number of main-line plies played before the line's first move.
CREATE TABLE variation (id INTEGER PRIMARY KEY, game_id INTEGER NOT NULL REFERENCES game(id),
  from_ply INTEGER NOT NULL CHECK (from_ply >= 0), pgn TEXT NOT NULL,
  created_at INTEGER NOT NULL);
CREATE TABLE analysis_job (game_id INTEGER PRIMARY KEY REFERENCES game(id),
  state TEXT NOT NULL CHECK (state IN ('queued','running','done','failed')),
  owner TEXT, heartbeat_at INTEGER);
CREATE TABLE game_tag (game_id INTEGER NOT NULL REFERENCES game(id), name TEXT NOT NULL, value TEXT NOT NULL,
  PRIMARY KEY (game_id, name)) WITHOUT ROWID;
CREATE TABLE game_lease (game_id INTEGER PRIMARY KEY REFERENCES game(id), owner TEXT NOT NULL, heartbeat_at INTEGER NOT NULL);
CREATE TABLE unfinished_slot (
  player_id INTEGER NOT NULL REFERENCES player(id), mode TEXT NOT NULL CHECK (mode IN ('training','strict')),
  game_id INTEGER NOT NULL REFERENCES game(id), PRIMARY KEY (player_id, mode)) WITHOUT ROWID;
CREATE TABLE app_setting (setting_key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;
CREATE TABLE process_heartbeat (owner TEXT PRIMARY KEY, build_version TEXT NOT NULL, heartbeat_at INTEGER NOT NULL) WITHOUT ROWID;
INSERT INTO player (id, uuid, kind, name, start_category, strict_elo, training_elo, created_at)
  VALUES (1, '00000000-0000-0000-0000-0000000000f1', 'human', 'Fixture Ann', 'Novice', 1200, 1200, 10),
         (2, '00000000-0000-0000-0000-0000000000f2', 'computer', 'Fixture Bot', 'Beginner', 800, 800, 11);
INSERT INTO preferences VALUES (1, '{"v":1}');
INSERT INTO app_setting VALUES ('roster_seed', '7');
INSERT INTO rating_history (player_id, track, rating_before, rating_after, k, at)
  VALUES (1, 'strict', 1200, 1200, 0, 10), (1, 'training', 1200, 1200, 0, 10);
INSERT INTO game (id, uuid, white_id, black_id, matchup, mode, rated, self_play, status, started_at)
  VALUES (1, '00000000-0000-0000-0000-0000000000e1', 1, 2, 'hc', 'strict', 1, 0, 'in_progress', 20);
INSERT INTO move (game_id, ply, san, uci, fen_after) VALUES (1, 1, 'e4', 'e2e4', 'fen-after-e4');
INSERT INTO unfinished_slot VALUES (1, 'strict', 1);
PRAGMA user_version = 1;
