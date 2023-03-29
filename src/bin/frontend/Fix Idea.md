# Decouple the ui state from the data

Essentially the app will contain two structs in it. A data struct that holds all the runtime data and a ui struct that holds the state of the ui.
That way the individual ui pieces can set themselves up how they need to and since the data is decoupled i can easily pass it into all the objects responsible for ui.