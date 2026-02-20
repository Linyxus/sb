case class Point(x: Double, y: Double):
  def distanceTo(other: Point): Double =
    math.sqrt(math.pow(x - other.x, 2) + math.pow(y - other.y, 2))

extension (points: List[Point])
  def centroid: Point =
    val n = points.size
    Point(points.map(_.x).sum / n, points.map(_.y).sum / n)

@main def app(): Unit =
  val pts = List(Point(0, 0), Point(3, 4), Point(6, 0))
  val c = pts.centroid
  println(s"Centroid: (${c.x}, ${c.y})")
  println(s"Distance 0->1: ${pts(0).distanceTo(pts(1))}")
