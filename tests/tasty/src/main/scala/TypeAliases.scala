// Type aliases and opaque types
object TypeAliases:
  type StringMap[V] = Map[String, V]
  type Predicate[A] = A => Boolean
  type Id[A] = A

  opaque type Email = String
  object Email:
    def apply(s: String): Email = s
    extension (e: Email)
      def domain: String = e.split("@").last
      def local: String = e.split("@").head

  opaque type NonNegInt = Int
  object NonNegInt:
    def apply(i: Int): Option[NonNegInt] =
      if i >= 0 then Some(i) else None
    extension (n: NonNegInt)
      def value: Int = n
      def +(other: NonNegInt): NonNegInt = n + other
