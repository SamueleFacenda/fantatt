package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.League
import org.springframework.data.repository.CrudRepository

interface LeagueRepository: CrudRepository<League, Long> {
}