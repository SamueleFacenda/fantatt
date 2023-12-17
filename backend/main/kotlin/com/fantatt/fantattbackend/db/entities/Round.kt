package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*
import java.sql.Date

@Entity
class Round (
    @ManyToOne
    val season: Season,
    val startDate: Date,
    val index: Int,
    @Id
    @GeneratedValue(strategy = GenerationType.IDENTITY)
    val id: Long?=null
)