import org.jetbrains.kotlin.gradle.tasks.KotlinCompile
import org.kordamp.gradle.plugin.jdeps.tasks.JDepsReportTask

sourceSets {
    main {
        kotlin.srcDirs("backend/main/kotlin")
        resources.srcDirs("backend/main/resources")
    }
    test {
        kotlin.srcDirs("backend/test/kotlin")
    }
}


plugins {
    id("org.springframework.boot") version "3.2.0"
    id("io.spring.dependency-management") version "1.1.4"
    kotlin("jvm") version "1.9.20"
    kotlin("plugin.spring") version "1.9.20"
    kotlin("plugin.jpa") version "1.9.20"
    id("org.kordamp.gradle.jdeps") version "0.20.0"
}

group = "com.fantatt"

java {
    sourceCompatibility = JavaVersion.VERSION_21
}

configurations {
    compileOnly {
        extendsFrom(configurations.annotationProcessor.get())
    }
}

repositories {
    mavenCentral()
}

// lock all configurations
dependencyLocking {
    lockAllConfigurations()
    lockMode = LockMode.STRICT
}

// lock plugins
buildscript {
    configurations.classpath {
        resolutionStrategy.activateDependencyLocking()
    }
}

dependencies {
    implementation("org.springframework.boot:spring-boot-starter-data-jpa")
    implementation("org.springframework.boot:spring-boot-starter-security")
    implementation("org.springframework.boot:spring-boot-starter-web")
    implementation("com.fasterxml.jackson.module:jackson-module-kotlin")
    implementation("org.jetbrains.kotlin:kotlin-reflect")
    compileOnly("org.projectlombok:lombok")
    developmentOnly("org.springframework.boot:spring-boot-devtools")
    runtimeOnly("com.h2database:h2")
    runtimeOnly("org.mariadb.jdbc:mariadb-java-client")
    annotationProcessor("org.projectlombok:lombok")
    testImplementation("org.springframework.boot:spring-boot-starter-test")
    testImplementation("org.springframework.security:spring-security-test")
}

tasks.withType<KotlinCompile> {
    kotlinOptions {
        freeCompilerArgs += "-Xjsr305=strict"
        jvmTarget = "21"
    }
}

tasks.withType<Test> {
    useJUnitPlatform()
}

tasks.withType<JDepsReportTask> {
    // run only if explicitly requested
    onlyIf { gradle.startParameter.taskNames.contains("jdeps") }
    // run every time
    outputs.upToDateWhen { false }
}

tasks.register("resolveDependencies") {
    doLast {
        rootProject.allprojects.forEach { project ->
            val configurations = project.buildscript.configurations + project.configurations
            configurations.filter { it.isCanBeResolved }.forEach { it.resolve() }
        }
    }
}
