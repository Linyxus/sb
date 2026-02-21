// Lazy vals
object LazyVal:
  lazy val expensive: Int =
    println("computing...")
    (1 to 1000).sum

  lazy val derived: String = s"result = $expensive"

  class LazyInit(threshold: Int):
    lazy val data: List[Int] = (1 to threshold).toList
    lazy val stats: (Int, Int, Double) =
      (data.min, data.max, data.sum.toDouble / data.length)
