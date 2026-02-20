object Utils:
  def summarize(items: List[Data]): String =
    val total = items.map(_.value).sum
    val names = items.map(_.name).mkString(", ")
    s"Items($names) total=$total"

  def maxByValue(items: List[Data]): Data =
    items.maxBy(_.value)
