case class Student(name: String, grade: Int, score: Double)

@main def collectionsMain(): Unit =
  val students = List(
    Student("Alice", 10, 92.5),
    Student("Bob", 10, 85.0),
    Student("Charlie", 11, 91.0),
    Student("Diana", 11, 78.5),
    Student("Eve", 10, 95.0),
    Student("Frank", 11, 88.0)
  )

  val byGrade = students.groupBy(_.grade)
  byGrade.foreach { (grade, ss) =>
    val avg = ss.map(_.score).sum / ss.size
    println(s"Grade $grade avg: $avg")
  }

  val (honors, regular) = students.partition(_.score >= 90)
  println(s"Honors: ${honors.map(_.name).mkString(", ")}")
  println(s"Regular: ${regular.map(_.name).mkString(", ")}")

  val ranked = students.sortBy(-_.score).zipWithIndex.map { (s, i) =>
    s"${i + 1}. ${s.name} (${s.score})"
  }
  ranked.foreach(println)

  val nameScoreMap = students.map(s => s.name -> s.score).toMap
  println(s"Alice's score: ${nameScoreMap("Alice")}")

  val totalScore = students.foldLeft(0.0)(_ + _.score)
  println(s"Total score: $totalScore")

  val nested = List(List(1, 2), List(3, 4), List(5))
  println(s"flatMap: ${nested.flatMap(_.map(_ * 10))}")
