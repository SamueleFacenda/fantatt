package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.LeagueEntity
import org.springframework.data.repository.CrudRepository

interface LeagueRepository: CrudRepository<LeagueEntity, Long> {
}