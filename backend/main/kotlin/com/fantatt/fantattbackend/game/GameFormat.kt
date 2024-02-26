package com.fantatt.fantattbackend.game

enum class GameFormat(
    val matches: List<Pair<String, String>>
) {
    SWAYTHLING(
        listOf(
            Pair("A", "X"),
            Pair("B", "Y"),
            Pair("C", "Z"),
            Pair("B", "X"),
            Pair("A", "Z"),
            Pair("C", "Y"),
            Pair("B", "Z"),
            Pair("C", "X"),
            Pair("A", "Y")
        )
    ),
    MINI_SWAYTHLING(
        listOf(
            Pair("A", "X"),
            Pair("B", "Y"),
            Pair("C", "Z"),
            Pair("B", "X"),
            Pair("A", "Z"),
            Pair("C", "Y")
        )
    ),
    NEW_SWAYTHLING(
        listOf(
            Pair("A", "X"),
            Pair("B", "Y"),
            Pair("C", "Z"),
            Pair("A", "Y"),
            Pair("B", "X"),
        )
    )
}