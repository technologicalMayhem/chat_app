This document tracks several things that are still an issue when using the application, or could just be improved in general.

- User Id's get reused. If a user with id 5 gets deleted and a new user registers they get id 5 and as such all messages belonging to the previous user now belong to them instead.
- Client does not close events connection properly. This is not a big issue but it does cause the server to crash, if it's shutting down, due to timeout if it still believes the client to be connected.
- Database does not get created if it does not already exist. This causes weird behaviour with the server as it doesn't actually have a database but it sort of believes it does? Not sure what's going on there.
- The handle_input function in the client is a bit of a mess. It's better than before but due to the way it structured it works really weird. Look into disjoint references or mutable aliasing.

