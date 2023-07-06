<meta x-title="Messing around with Perkeep"/>

[Perkeep](https://perkeep.org/) is a project authored by [Brad Fitzpatrick](https://twitter.com/bradfitz) (of Go fame). It aims to be a good solution to long term, self-hosted, personal data storage and ticks many of the boxes I’ve been looking for.

![perkeep](perkeep.png)

For a long time I’ve been looking for a good data storage solution that combines storage of data files (personal documents, photos, etc) and ties in nicely with the whole idea of “quantified-self”. I’d like to be able to store things like financial records (double entry accounting entries); health records like weight, exercise, runs; gps logs; and most critically, be in full control of the data and be able to migrate it between platforms and build projects on top of this data.

My current solution to this is a hodge-podge mix of SpiderOak Hive, Google Photos, Dropbox, Dropbox Paper and I’m just not happy with it: I feel like having full control of it would be better privacy wise and being able to contribute to Perkeep opens up many possibilities.

I dabbled with building some of this stuff myself, old Github projects tell of Rails-based financial tracking web-apps, Java-based encrypted file stores, many ideas have been drawn-up, started, abandoned and subsequently scrapped (like 99.9% of things on Github).

However, yesterday, I stumbled upon the Perkeep (neé Camlistore) project. It is very intriguing and the documentation and compare page are particularly attractive.

> Things Perkeep believes:
> 
> - Your data is entirely under your control
> - Open Source
> - Paranoid about privacy, everything private by default
> - No SPOF: don’t rely on any single party (including yourself)
> - Your data should be alive in 80 years, especially if you are

So, for the remainder of this blog post, I’ll be setting up some remote storage and experimenting with a local server for a bit. I’m going into this hoping that I’ll be able to move the bulk of my data to it in the future.

## Cloud Storage Setup

Perkeep is able to use a number of different backend providers for blob storage, I’m going to go with an AWS S3 object store bucket simply because I’m already using AWS EC2 and I’m familiar with it.

So the first step will be setting up an IAM group and user for this project.

1. Created a new IAM group `PerkeepGroup`
2. Created a new IAM user `PerkeepStorage` in the group (will enforce permissions shortly)
3. Created a new S3 bucket `perkeep-personal-storage`
4. Added an IAM policy limiting the `PerkeepStorage` user to just the perkeep-personal-storage bucket. `(arn:aws:s3:::perkeep-primary-storage/*)`

Testing the auth:

```
$ touch blerp
$ aws configure
$ aws s3 cp blerp s3://perkeep-personal-storage/blerp
upload: ./blerp to s3://perkeep-personal-storage/blerp
$ aws s3 rm s3://perkeep-personal-storage/blerp
```

**Note:** the reason I’m using a separate user for this S3 bucket is that the key needs to be embedded in the configuration of the Perkeep server or other clients. It would be a bad idea to be embedding a root credential there.

### Running Perkeep locally

Before going all the way to a Perkeep instance, I felt it prudent to mess around with it locally on my laptop.

- I first thought I could use the download from the 2017–05 release, but it didn’t contain the server binary, and seemed generally a bit old for a project that is currently actively maintained and has recently been renamed (Camliproject → Perkeep). So I bit the bullet and pulled the Golang source for a local build.
- Then it complained that I needed Golang 1.10 although docs only stated 1.9.. 😑
- At least the dependencies are vendored so my South African internet doesn’t have to sweat too much 👍
- `go run make.go` worked fine and running the binary ran without issues.

I configured the server as follows:

```
{
    "auth": "userpass:dummy:dummy",
    "listen": ":3179",
    "camliNetIP": "",
    "identity": "XXXXXXXXXXXXXXXX",
    "identitySecretRing": "/Users/benmeier/.config/perkeep/identity-secring.gpg",
    "packRelated": true,
    "levelDB": "/Users/benmeier/Library/Camlistore/index.leveldb",
    "s3": "someid:somekey:perkeep-personal-storage"
}
```

**Note:** at this point I hit bug #911 because my bucket was in EU Frankfurt region which apparently mandates a new auth mechanism. So until this is fixed in Perkeep you need to host the bucket elsewhere like EU Ireland

**Note 2:** I also found that the example IAM config in Perkeep’s docs was a bit out of date and needed some additions for permissions on the bucket itself, not just the `/*`.

## Results

I spent a few hours uploading images and files to the local server and experimenting with the UI. I had a few sticky points getting uploads to work correctly but I think most problems were due to a slow and unreliable internet connection. The following are my thoughts after messing around with it. _Please bear in mind that these issues are only temporary and may be fixed or improved in future:_

- It takes a bit to get used to the fact that this is NOT file-oriented. You have “Sets” which are kind of like folders and can be nested, but the tree is quite unlike a normal file browser.
- The search is fairly non-intuitive. It’s certainly powerful, but I’d expect that when you type a simple string, it’ll pick up results from “Sets” that have the similar name. So I’m trying to think about how I’d navigate the UI when using it in the day to day.
- PDFs aren’t really supported as a type, if you open the permanode for one you get a “Content not found” message which is a bit counter intuitive. To download it, you have to go back to the main ui > context menu > download original.
- No support for easily renaming “files” or “sets” yet. It’s possible, but really tricky in the UI.
- No UI sorting, attribute columns, etc. Feels like a toolbar would be useful.
- Images seem to work really well, but almost everything else seems like a second-class citizen in the UI.
- Upload dialog was confusing, I know the JS upload progress API’s are a bit “meh” but I’d expect the upload progress to be a bit more indicative of the speed/bandwidth.
- I wonder how the UI holds up when you have thousands of items in it. It feels like you’d quickly become much more dependant on the search functionality rather than any kind of “file-system”-like views. I’m imagining a varied mix of high-res photos, pdf documents, text files, tweets, rss feeds, links, json facts, etc.. all with different varieties of importance and read/write balance.

In general, I really like the concept of the storage engine. It is super powerful and has almost endless possibilities. However I went into the experiment wanting to find a ready-to-go system that I could begin using immediately without much worry on an EC2 instance, and I feel like I ended up with something that I want to contribute to first before I can begin using it. (And attempt to help fix some of the issues I had). Guess it’s time to learn some React! 😁
