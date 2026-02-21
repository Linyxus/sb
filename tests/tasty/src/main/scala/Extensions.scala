// Extension methods
object Extensions:
  extension (s: String)
    def words: Array[String] = s.split("\\s+")
    def capitalize: String = if s.isEmpty then s else s(0).toUpper + s.substring(1)
    def repeat(n: Int): String = s * n

  extension [A](list: List[A])
    def second: Option[A] = list match
      case _ :: x :: _ => Some(x)
      case _ => None
    def penultimate: Option[A] = list.reverse.second
