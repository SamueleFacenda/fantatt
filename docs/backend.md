# How does this work/what does it need to do

You can subscribe as a single user (email + password)
A league is created by a single user, that insert all the members teams and credits.
There should be a player list (or a query player box).
Every week a score is given to each player (or just the used ones), it will also
be good to preview it
A match result must be computed, using the team plan given by the players. 
A team plan (formazione, non so come si dica in inglese), should contain three(?) main players
and and order of reserves (that are deployed if a main has not played).
If a plan is not submitted the one from the last week is used (or a random/automatic one).
For every league there is a score for each manager, a scoreboard, an history(next step) and
a match calendar.
A double italian round is used (gone and return?)

In the db, the player in a team has an order number, the lower three
players of a team (1,2,3) are the main (X,Y,Z or A,B,C),
the other ordered are the reserves.

One choice, the game data (results, current status, etc.) are saved in
the db or computed everytime? They are lightweight to cumpute,
but it's not the best decision.

## DB notes (integrity clauses)
Cheks not embebbed in the db schema
A player can belong to more teams, but only one team per league.
The season of a round of a match must be the same of the league of the teams playing. (hard to check, useless)

