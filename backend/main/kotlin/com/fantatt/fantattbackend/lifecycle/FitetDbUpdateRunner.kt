package com.fantatt.fantattbackend.lifecycle

import com.fantatt.fantattbackend.db.matches.SqliteFitetParsedMatchRepository
import org.springframework.beans.factory.annotation.Autowired
import org.springframework.scheduling.annotation.Scheduled
import java.io.File
import kotlin.io.path.Path
import kotlin.io.path.absolutePathString
import kotlin.io.path.copyTo
import kotlin.io.path.deleteExisting


const val UPDATE_CMD = "python -m fitet --update --dump-path %s"
@Autowired
lateinit var sqliteMatchRepository: SqliteFitetParsedMatchRepository
@Autowired
lateinit var schedulingConfig: SchedulingConfig
// one time per hour (maybe not)
@Scheduled(cron = "0 2 * * * *")
fun updateDb() {
    // update a copy of the database to block the access to the database for less
    // time (just the time to copy the file)
    val db = Path(schedulingConfig.dumpPhat)
    val tmpDb = Path(schedulingConfig.dumpPhat + ".tmp")
    db.copyTo(tmpDb, true)
    Runtime.getRuntime().exec(UPDATE_CMD.format(tmpDb.absolutePathString()))

    sqliteMatchRepository.update {
        tmpDb.copyTo(db, true)
    }

    tmpDb.deleteExisting()
}