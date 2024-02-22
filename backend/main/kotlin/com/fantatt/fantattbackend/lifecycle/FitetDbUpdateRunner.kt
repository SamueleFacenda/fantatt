package com.fantatt.fantattbackend.lifecycle

import com.fantatt.fantattbackend.db.matches.SqliteFitetParsedMatchRepository
import org.springframework.beans.factory.annotation.Autowired
import org.springframework.scheduling.annotation.Scheduled
import java.io.File


const val UPDATE_CMD = "python -m fitet --update --dump-path %s"
@Autowired
lateinit var sqliteMatchRepository: SqliteFitetParsedMatchRepository
@Autowired
lateinit var schedulingConfig: SchedulingConfig
// one time per hour
@Scheduled(cron = "0 2 * * * *")
fun updateDb() {
    // update a copy of the database to block the access to the database for less
    // time (just the time to copy the file)
    val db = File(schedulingConfig.dumpPhat)
    val tmpDb = File(schedulingConfig.dumpPhat + ".tmp")
    db.copyTo(tmpDb, true)
    Runtime.getRuntime().exec(UPDATE_CMD.format(tmpDb.path))

    sqliteMatchRepository.startUpdate()
    tmpDb.copyTo(db, true)
    sqliteMatchRepository.endUpdate()

    tmpDb.delete()
}