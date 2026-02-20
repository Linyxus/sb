import cats.syntax.all.*
import cats.Monoid

@main def multiDepMain(): Unit =
  // Use os-lib to get temp dir info
  val cwd = os.pwd
  println(s"Current directory: $cwd")

  val tmpDir = os.temp.dir()
  os.write(tmpDir / "test.txt", "hello from multi_dep")
  val content = os.read(tmpDir / "test.txt")
  println(s"Wrote and read: $content")

  // Use cats Monoid to combine results
  val lines = List("alpha", "beta", "gamma")
  lines.foreach(line => os.write.append(tmpDir / "combined.txt", line + "\n"))
  val allLines = os.read.lines(tmpDir / "combined.txt").toList
  val combined = allLines.foldMap(s => s"|$s|")
  println(s"Combined with cats foldMap: $combined")

  // Cleanup
  os.remove.all(tmpDir)
  println("Done")
