extension (s: String)
  def exclaim: String = s + "!"
  def repeat(n: Int): String = s * n
  def words: List[String] = s.split("\\s+").toList

extension (n: Int)
  def isEven: Boolean = n % 2 == 0
  def factorial: BigInt =
    (1 to n).foldLeft(BigInt(1))(_ * _)

@main def extensionsMain(): Unit =
  println("hello".exclaim)
  println("ha".repeat(3))
  println("the quick brown fox".words)
  println(s"10! = ${10.factorial}")
  println(s"7 is even: ${7.isEven}")
