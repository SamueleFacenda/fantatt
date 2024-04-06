package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*
import java.time.LocalDateTime

@Entity
class RoundEntity (
    @ManyToOne
    val season: SeasonEntity,
    val startTime: LocalDateTime,
    val endTime: LocalDateTime,
    val resultTime: LocalDateTime, // results should be available at this time (e.g. 2-3 days)
    val index: Int, // 0-indexed
    @Id @GeneratedValue val id: Long?=null
)