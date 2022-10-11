# Styx DAO
The Scrypto DAO to make the most of Cerberus

# Functionalities
The DAO can hold assets and be associated to another blueprint that can emit DAO tokens using
an admin badge.  
Users can interact with the DAO by using a VoterCard. They can propose to change the
parameters of voting, emit new DAO tokens or spend the assets under management.

# Voting System
The DAO implements a new liquid staking mechanism.To vote or delegate their tokens, users have
to lock their tokens into the DAO.  
When a token is locked, its voting power is equal to 0 and grows to reach 1 with time.
The total voting power is then corrected using another function to make sure that no one is too
powerful.
# Whitepaper
For more details, please read the whitepaper