package com.fantatt.fantattbackend.web

import com.fantatt.fantattbackend.db.entities.LeagueEntity
import com.fantatt.fantattbackend.db.entities.SocietyEntity
import com.fantatt.fantattbackend.db.repos.LeagueRepository
import com.fantatt.fantattbackend.db.repos.UserRepository
import org.springframework.web.bind.annotation.GetMapping
import org.springframework.web.bind.annotation.PathVariable
import org.springframework.web.bind.annotation.ResponseBody
import org.springframework.web.bind.annotation.RestController
import java.security.Principal

@RestController
class DataFetchController(
    val userRepository: UserRepository,
    val leagueRepository: LeagueRepository
) {

    @GetMapping("/user/leagues")
    @ResponseBody
    fun getUserLeagues(principal: Principal): List<LeagueEntity> {
        return userRepository.findByUsername(principal.name)
            ?.societies?.map { it.league } ?: throw Exception("User not found")
    }

    @GetMapping("league/{leagueId}/teams")
    @ResponseBody
    fun getLeagueTeams(@PathVariable leagueId: Long): List<SocietyEntity> {
        return leagueRepository.findById(leagueId).orElseThrow { Exception("League not found") }.societies
    }


    // get team result

    // get society result

    // get team result for round

    // get society result for round
}