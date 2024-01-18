# General architecture of fantatt

You should already know how tabletennis (fitet rules) and a fantasy league works.
The project is divided in three parts:
- the frontend (react js)
- the backend (spring boot kotlin)
- the data fetcher (python)


## Data fetching
The data are fetched from the fitet official results portal. Since there is not an API and the entire website use 
legacy technologies like framesets and so, it's a mess of html parsing, web scraping, codes and ids moving.

The implementation uses python requests and beautifulsoup4 to get the data and sqlalchemy to persist them. 
Quick updating is supported (fetch just the changes) and persistent caching is used where possible.

## Data sources (DB)
There are two db, the one with the data from the fitet portal and the application data one (users, fantasy league data).
The first is created with sqlalchemy orm and I plan to use it from spring boot with the spring data JDBC library 
(language agnostic sql). The latter should be created and managed with the spring data JPA orm. In spring boot is easy to define
more data repositories.

## Infra
Nixos!!
All the parts are already ready to be packaged with nix, so the deployment shoud be fast and easy.
Defining the server configuration with nixos shoud also be very easy for me (containers).

## Frontend deployment
Requirements: caching of the statis folder (react output with hash names)
Possibilities:
- nginx reverse proxy, mask the api and the react assets under one endopoint
- spring boot assets serving, configurable via env vars, supports caching but not so configurable
(I cannot not cache just the index.html)

## About the game

### Teams and logic
I'm assuming three people teams with a bench of currently 5
players, and maybe some auto-switch logic (take from bench if
the starting player is not playing or from auto-switch if the switch
has better score).
Edit:
We want something more here. Each user is the manager of a society, each society has
3 teams. There are three categories (A, B and C), at the beginning of a league each
society has a team per group (category), when all the matches are played (a 'rotation'),
a number of teams (depending on the number of teams in a group, usually two) changes group.
The n top goes up, the n worse goes down.
The final scoreboard (between societies) will be ordered by a score that is the sum of the scores
of the three teams. The score of each team will be multiplied by a category-weight, that can be
, for example:
- A - 2.0
- B - 1.0
- C - 0.75
Thinking about the weight is important to remember that: the rotations are pretty often, rising category
and beeing the last of the higher one should be good. The last category should have some weight, 
in order to not make if the 'trash' category.

Of course every week a society's teams will play agains more than one society, derby are also possible
(more realistic trait).
Question: Are category changes allowed or we will follow the fitet regulamentation (allowed only after rotation or
upward)?

### Rounds
Each round is one week long, it starts and end on thursday midnight.
The easiest think is to not follow the championship calendar, but to
just use weeks, because there are several championships with different
calendars. And also when all the championships have a pause, there are 
tournament. So I will probably implement just a season start, end, season
pause start and end date mechanism.
