package com.fantatt.fantattbackend.web

import com.fantatt.fantattbackend.db.entities.League
import com.fantatt.fantattbackend.db.entities.Team
import com.fantatt.fantattbackend.db.game.LeagueCreator
import org.springframework.web.bind.annotation.PostMapping
import org.springframework.web.bind.annotation.RequestParam
import org.springframework.web.bind.annotation.RequestBody
import org.springframework.web.bind.annotation.ResponseBody
import org.springframework.web.bind.annotation.RestController
import java.security.Principal

@RestController
class GameController(
    val leagueCreator: LeagueCreator
) {

    @PostMapping("/league/create")
    @ResponseBody
    fun createLeague(@RequestParam name: String, @RequestBody teams: List<Team>, principal: Principal): League {
        return leagueCreator.leagueFrom(
            masterName = principal.name,
            leagueName = name,
            teams = teams
        )
    }


}